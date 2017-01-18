rfc
===

Frequently Asked Questions
--------------------------
What is this?
> A little helper tool we use for managing some of the tedious parts of mimicking Rust's RFC process for our own projects.

How do I get it?
> `cargo install rfc`

And how does it compare to the ideal tool for managing that process? Like, what word(s) would you use to fill in the _blank_ in the statement "`rfc` is a _blank_ of the platonic ideal of an RFC manager." ?
> "tepid, misguided disciple"

How do I find out the tepid things it claims to do?
> `rfc --help`

Sure, but where do I _start?_
> In a git-managed directory with a `README.md` file, do `rfc init` to create the `rfcs` directory and to add an `Active RFCs` section to your README. From a clean `master`, start a new rfc with `echo my-fancy-rfc-name | rfc new`.

Are there tons of things it should probably do that it doesn't?
> Yeap

Are there tests?
> Like the SAT? Um...what? No.

Haha! Too funny! But really, if something _appears_ broken with the tool—for instance, `rfc new` doesn't seem to do anything—where do I go to see what the correct/expected behavior is?
> Huh...I guess you could try submitting an rfc to `rfc` itself outlining _what the behavior should be_. Just do `rfc new`.

Was it written by developers very new to Rust?
> Definitely!

If they were to re-write it now from scratch, would it be far more idiomatic and elegant?
> Not as much as they'd have you believe!

How likely is the interface to change between releases without notice?
> Very

Is the unadorned, simple name the result of the authors taking advantage of crates.io's first-come/first-serve flat namespace?
> Totes! Gotta love it!

# Active RFCs<!--- auto-generated section -->
