use regex::{self, Regex};

fn greater_or_equal_header_level_regex(level: usize) -> Regex {
    let string = match level {
        0 => panic!("Can't have a 0-level header"),
        1 => "(?m)^# +[^\\s].*\\n|(?m)^[^\\s].*\\n=+\\n".to_string(),
        _ => format!("(?m)^#{{1,{}}} +[^\\s].*\\n|(?m)^[^\\s].*\\n(=+|-+)\\n", level),
    };

    Regex::new(&string).expect("Header regex failed to compile")
}

fn header_regex(header: &str) -> Regex {
    Regex::new(&format!("(?m)^(#+) +{0}\\n|(?m)^{0}\\n(-+|=+)\\n", regex::quote(header)))
        .expect("Header regex failed to compile")
}

pub fn find_section(markdown: &str, header: &str) -> Option<(usize, usize)> {
    let header_regex = header_regex(header);

    let (header_start, content_start) = match header_regex.find(markdown) {
        Some((a, b)) => (a, b),
        None => return None,
    };

    let header_level = header_level(markdown, header).unwrap();

    let next_header_start = match greater_or_equal_header_level_regex(header_level).find(&markdown[content_start..]) {
        None => markdown.len(),
        Some((a, _)) => a + content_start,
    };

    Some((header_start, next_header_start))
}

pub fn get_section(markdown: &str, header: &str) -> Option<String> {
    if let Some((section_start, section_end)) = find_section(markdown, header) {
        Some(String::from(&markdown[section_start..section_end]))
    } else {
        None
    }
}


pub fn replace_section(markdown: &str, header: &str, insertion: &str) -> Option<String> {
    let section = match get_section(markdown, header) {
        Some(string) => string,
        None => return None,
    };

    let header_text = header_regex(header)
        .captures(markdown).unwrap()
        .at(0).unwrap()
        .clone();

    let full_replacement_text = format!("{}{}", header_text, insertion);

    Some(markdown.replace(&section, &full_replacement_text))
}

pub fn replace_or_append_section(markdown: &str, header: &str, insertion: &str) -> String {
    match replace_section(markdown, header, insertion) {
        Some(string) => string,
        None => format!("{}\n# {}{}", markdown, header, insertion),
    }
}

pub fn header_level(markdown: &str, header: &str) -> Option<usize> {
    match header_regex(header).captures(markdown) {
        Some(captures) => {
            Some(if let Some(num_signs) = captures.at(1) {
                num_signs.chars().count()
            } else if let Some(horiz_line) = captures.at(2) {
                match &horiz_line[..1] {
                    "=" => 1,
                    "-" => 2,
                    _ => unreachable!(),
                }
            } else {
                unreachable!()
            })
        }
        None => None,
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_markdown_replace() {
        let md = "\
          abc\
          \n\
          \n```\
          \n  123\
          \n```\
          \n\
          \n#### Blarg\
          \nblergins\
          \n- [85](lakjlkja)\
          \n\
          \n###### Florm\
          \n\
          \n###### Florm 2\
          \n\
          \n# Blaag\
          \nFlerbin Storpl\
          \n---\
          \n\
          \nJaxr\
          \n===\
          \n\
          \n\
          \nJaaaz\
          \n\
          \n\
          \n\
        ";

        assert_eq!(super::header_level(&md, "Blarg"), Some(4));
        assert_eq!(super::header_level(&md, "Flerbin Storpl"), Some(2));
        assert_eq!(super::header_level(&md, "Jaxr"), Some(1));
        assert_eq!(super::header_level(&md, "Jaaaz"), None);

        assert_eq!(super::get_section(&md, "Jaxr"), Some("Jaxr\n===\n\n\nJaaaz\n\n\n".to_string()));
        assert_eq!(super::get_section(&md, "Flerbin Storpl"), Some("Flerbin Storpl\n---\n\n".to_string()));
        assert_eq!(super::get_section(&md, "Blaag"), Some("# Blaag\nFlerbin Storpl\n---\n\n".to_string()));
        assert_eq!(super::get_section(&md, "Not Real"), None);

        assert_eq!(super::replace_or_append_section(&md, "Blarg", "abc\n123\n"),
                   "\
                     abc\
                     \n\
                     \n```\
                     \n  123\
                     \n```\
                     \n\
                     \n#### Blarg\
                     \nabc\
                     \n123\
                     \n# Blaag\
                     \nFlerbin Storpl\
                     \n---\
                     \n\
                     \nJaxr\
                     \n===\
                     \n\
                     \n\
                     \nJaaaz\
                     \n\
                     \n\
                     \n\
                   ".to_string());
    }
}
