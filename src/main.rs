#[macro_use]
extern crate lazy_static;
extern crate regex;
use regex::Regex;
extern crate docopt;
extern crate time;

mod markdown;

pub static GIT_COMMIT_HASH: &'static str = include_str!(concat!(env!("OUT_DIR"), "/git_commit_hash.txt"));
pub static CRATE_VERSION: &'static str = include_str!(concat!(env!("OUT_DIR"), "/crate_version.txt"));
pub static COMPILATION_TIMESTAMP: &'static str = include_str!(concat!(env!("OUT_DIR"), "/compilation_timestamp.txt"));

static USAGE: &'static str = "
Collection of utilities for managing the rfc process.

Usage:
    rfc new
    rfc init
    rfc list (active | pending)
    rfc approve <rfc-id> <pr-id>
    rfc implement <rfc-id> <pr-id>
    rfc (-h | --help)
    rfc (-v | --version)

Options:
    -h, --help      Show this message
    -v, --version   Print version information and exit
";

use docopt::Docopt;
use std::fs::{self, File};
use std::io::{self, BufReader, BufRead, Read, Write};
use std::env;
use std::ascii::AsciiExt;

static RFC_DIR: &'static str = "./rfcs";

lazy_static! {
    static ref PR_PATH: String = pull_request_path();
}

lazy_static! {
    static ref PROJECT_NAME: String = project_name();
}

static DEFAULT_TEMPLATE: &'static str = include_str!("../templates/default.md");

use std::process::Command;

macro_rules! cmd {
    ( $prog:expr, $($arg:expr),* ) => {{
        run_command(Command::new($prog)
                    .args(&[
                          $( $arg, )*
                    ]))
    }}
}

macro_rules! map_err_display {
    ($thing:expr) => { $thing.map_err(|err| format!("{:}", err)) }
}

macro_rules! expect_file_line {
    ($thing:expr) => { $thing.expect(&format!("{}:{}", file!(), line!())) }
}

fn pull_request_path() -> String {
    env::var("PR_PATH").ok()
        .or_else(|| {
            cmd!("git", "config", "--get", "remote.origin.url").ok()
                // TODO improve the following logic
                .and_then(|url| {
                    let rx = Regex::new(r#"\Agit@github\.com:(.+)\.git(?:\r?\n)?\z"#).unwrap();
                    rx.captures(&url).map(|captures| {
                        String::from("https://github.com/")
                            + captures.at(1).unwrap()
                            + "/pull"
                    })
                })
        })
        .unwrap_or_default()
}

fn project_name() -> String {
    env::var("PROJECT_NAME").ok()
        .unwrap_or_else(|| {
            env::current_dir().unwrap()
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap()
        })
}

fn main() {
    let args = Docopt::new(USAGE)
                      .and_then(|dopt| dopt.parse())
                      .unwrap_or_else(|e| e.exit());

    if args.get_bool("-v") || args.get_bool("--version") {
        print_version_information();
        return;
    }

    if args.get_bool("new") {
        new_rfc();
        return;
    }

    if args.get_bool("init") {
        init_rfcs();
        return;
    }

    if args.get_bool("list") {
        if args.get_bool("active") {
            let active = list_active_rfcs();
            if active.len() == 0 { println!("No active rfcs.")}
            else { println!("{}", list_active_rfcs().join("\n")) }
        }

        if args.get_bool("pending") {
            let pending = list_pending_rfcs();
            if pending.len() == 0 { println!("No pending rfcs.") }
            else { println!("{}", pending.join("\n")) }
        }

        return;
    }

    if args.get_bool("approve") {
        // TODO Grab PR id from Github if none is provided
        expect_file_line!(approve(args.get_str("<rfc-id>"), args.get_str("<pr-id>")));
        return;
    }

    if args.get_bool("implement") {
        // TODO Grab PR id from Github if none is provided
        expect_file_line!(implement(args.get_str("<rfc-id>"), args.get_str("<pr-id>")));
        return;
    }
}

/// Move pending rfc to active state.
/// This should be used after an rfc pull request has been submitted and discussed.
fn approve(id: &str, pr_id: &str) -> Result<(), String> {
    // Perform an existence check for the supplied rfc-id
    { try!(map_err_display!(File::open(rfc_path(&id)))); };

    // Abort unless working tree is pristine
    if !try!(cmd!("git", "status", "-z")).is_empty() {
        return Err("\
                   Cannot merge RFC until working tree is clean.\
                   \nCommit or stash all changes\
                   \n".to_string());
    }

    let branch = try!(cmd!("git", "rev-parse", "--abbrev-ref", "HEAD"))
        .trim().to_string();

    try!(cmd!("git", "fetch", "origin"));
    try!(cmd!("git", "checkout", "master"));
    try!(cmd!("git", "pull", "--rebase=preserve"));

    try!(cmd!("git", "merge", "--no-commit", &branch));

    try!(populate_rfc_pr(&rfc_path(&id), "RFC PR", &pr_id));
    try!(cmd!("git", "add", &rfc_path(&id)));

    let new_rfc_id = incr_rfc_id(id);
    try!(cmd!("git", "mv", &rfc_path(id), &rfc_path(&new_rfc_id)));

    try!(update_readme());
    try!(cmd!("git", "add", README_PATH));

    try!(cmd!("git", "commit", "--message", &format!("Approve RFC {}", id)));

    Ok(())
}

fn implement(id: &str, pr_id: &str) -> Result<(), String> {
    // Perform an existence check for the supplied rfc-id
    { try!(map_err_display!(File::open(rfc_path(&id)))); };

    // Abort unless working tree is pristine
    if !try!(cmd!("git", "status", "-z")).is_empty() {
        return Err("\
                   Cannot mark RFC as implemented until working tree is clean.\
                   \nCommit or stash all changes\
                   \n".to_string());
    }

    try!(populate_rfc_pr(&rfc_path(&id), "Implementation PR", &pr_id));
    try!(cmd!("git", "add", &rfc_path(&id)));

    try!(update_readme());
    try!(cmd!("git", "add", README_PATH));

    try!(cmd!("git", "commit", "--message", &format!("Implement RFC {}", id)));

    Ok(())
}

fn populate_rfc_pr(filename: &str, tag: &str, pr_id: &str) -> Result<(), String> {
    let mut buffer = {
        let mut file = try!(map_err_display!(File::open(filename)));
        let mut buffer = String::with_capacity(try!(map_err_display!(file.metadata())).len() as usize);
        try!(map_err_display!(file.read_to_string(&mut buffer)));
        buffer
    };

    let project_name: &String = &PROJECT_NAME;
    let pr_path: &String = &PR_PATH;

    buffer = buffer.replace(&format!("{}: (leave this empty)", tag),
                            &format!("{tag}: [{project_name}#{pr_id}]({pr_path}/{pr_id})",
                            tag = tag,
                            project_name = project_name,
                            pr_path = pr_path,
                            pr_id = pr_id));

    let mut out_file = File::create(filename.clone()).unwrap();
    map_err_display!(out_file.write_all(&buffer.into_bytes()))
}

fn run_command(command: &mut Command) -> Result<String, String> {
    let output = try!(map_err_display!(command.output()));

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout[..]).into_owned())
    } else {
        Err(String::from_utf8_lossy(&output.stderr[..]).into_owned())
    }
}

/// Turns a string like `0000-SOME-DESCRIPTION` into
/// `000X-SOME-DESCRIPTION`
fn incr_rfc_id(id: &str) -> String {
    let parts: Vec<&str> = id.splitn(2, '-').collect();
    let rest = parts[1];
    format!("{:04}-{}", max_accepted_rfc_num() + 1, rest)
}

fn rfc_path(id: &str) -> String {
    format!("{}/{}.md", RFC_DIR, id)
}

/// Fill in template to begin the rfc process
fn new_rfc() {
    let mut read_buffer = String::new();
    match io::stdin().read_line(&mut read_buffer) {
        Ok(0) => return,
        Ok(_) => {},
        Err(_) => return,
    }

    let re = regex::Regex::new(r###"([\s_]+)"###).unwrap();
    let name0 = read_buffer.trim().to_ascii_lowercase();
    let name = re.replace_all(&name0[..], "-");

    cmd!("git", "checkout", "master").unwrap();
    cmd!("git", "checkout", "-b", &rfc_branch_name(&name)).unwrap();

    let timestamp = time::strftime("%Y-%m-%d", &time::now()).unwrap();
    let mut template_file = File::open(RFC_DIR.to_owned() + "/0000-template.md").unwrap();
    let mut template = String::new();
    template_file.read_to_string(&mut template).unwrap();

    template = template.replace("(fill me in with a unique ident, my_awesome_feature)", &name[..]);
    template = template.replace("(fill me in with today's date, YYYY-MM-DD)", &timestamp[..]);

    let out_filename = format!("{}/0000-{}.md", RFC_DIR, name);
    let mut out_file = File::create(out_filename.clone()).unwrap();
    out_file.write_all(&template.into_bytes()).unwrap();

    cmd!("git", "add", "--intent-to-add", &out_filename).unwrap();

    println!("{}", out_filename);
}

fn init_rfcs() {
    setup_rfcs_dir();
    update_readme()
        // Remove partial work if updating readme fails
        .or_else(|_| fs::remove_dir_all("rfcs")).unwrap();
}

fn setup_rfcs_dir() {
    fs::create_dir("rfcs").unwrap();
    let mut file = fs::File::create("rfcs/0000-template.md").unwrap();
    let mut buffer: Vec<u8> = DEFAULT_TEMPLATE.bytes().collect();
    file.write_all(&mut buffer).unwrap();
}

fn rfc_branch_name(name: &str) -> String {
    format!("rfc-{}", name)
}

/// Read through `rfcs` dir and return all rfcs that are not assigned a number
fn list_pending_rfcs() -> Vec<String> {
    let paths = fs::read_dir(RFC_DIR).unwrap();
    let mut acc = vec!();
    for path in paths {
        let p = format!("{}",
                        path.unwrap().path().file_name().unwrap()
                        .to_string_lossy()
                        .rsplitn(2, '.').nth(1).unwrap());
            if !p.contains("0000-") { continue }
        if p.contains("0000-template") { continue }
        acc.push(p)
    }
    acc
}

/// Read through `rfcs` dir and return all rfcs that are assigned a number
fn list_accepted_rfcs() -> Vec<String> {
    fs::read_dir(RFC_DIR).unwrap()
        .map(|path| format!("{}",
                            path.unwrap()
                            .path().file_name().unwrap()
                            .to_string_lossy()
                            .rsplitn(2, '.').nth(1).unwrap())
            )
        .filter(|path| !path.starts_with("0000-"))
        .collect()
}

/// Read through `rfcs` dir and return all rfcs that are assigned a number
fn list_accepted_rfc_nums() -> Vec<String> {
    list_accepted_rfcs().into_iter()
        .map(|path| String::from(path.splitn(2, '-').next().unwrap()))
        .collect()
}

fn is_implemented(filename: &str) -> bool {
    let file = File::open(filename)
        .expect(&format!("Unable to open {}", filename));

    for line in BufReader::new(file).lines() {
        let line = expect_file_line!(line);

        if line.starts_with("- Implementation PR: ") {
            return !line.ends_with("(leave this empty)");
        }
    }

    panic!("No `- Implementation PR: ...` line found in {}", filename)
}

fn list_active_rfcs() -> Vec<String> {
    list_accepted_rfcs().into_iter()
        .filter(|ref string| !is_implemented(&rfc_path(&string)))
        .collect()
}

fn markdown_link_list(rfcs: &Vec<(String, String)>) -> Vec<String> {
    rfcs.iter()
        .map(|&(ref rfc, ref path)| format!("- [{}]({})\n", rfc, path))
        .collect()
}

fn markdown_active_rfcs() -> String {
    markdown_link_list(
        &list_active_rfcs().into_iter()
        .map(|ref rfc| (rfc.clone(), rfc_path(rfc)))
        .collect::<Vec<(String, String)>>()
        ).into_iter().collect()
}

static README_PATH: &'static str = "README.md";

fn update_readme() -> Result<(), String> {
    let mut buffer = {
        let mut file = try!(map_err_display!(File::open(README_PATH)));
        let mut buffer = String::new();
        try!(map_err_display!(file.read_to_string(&mut buffer)));
        buffer
    };

    buffer = markdown::replace_or_append_section(
        &buffer, "Active RFCs",
        &format!("<!--- auto-generated section -->\n{}", markdown_active_rfcs())
        );

    let mut out_file = expect_file_line!(File::create(README_PATH));
    map_err_display!(out_file.write_all(&buffer.into_bytes()))
}

fn max_accepted_rfc_num() -> usize {
    list_accepted_rfc_nums().into_iter().map(|string_num| {
        string_num.parse::<usize>().unwrap()
    }).max().unwrap()
}

fn print_version_information() {
    println!("rfc {} ({}) ({})", CRATE_VERSION, GIT_COMMIT_HASH, COMPILATION_TIMESTAMP);
}
