# AGENTS.md (Rust version)

## The app

This app extracts the English subtitles from video files and uses OpenAI to translate them.
Special features:

* Subtitles with context: the AI looks at the video title to understand the topic. This helps it correctly translate terms that vary by context.
* Timing preserved: only the text content of one line is changed at a time, with backward and forward context (4 lines translated and not translated before, and 4 lines after). This prevents lines from getting mixed (no translation from a previous line leaking into another).
* SRT integrity check at the end with a simple parser.
* Extracts English subtitles using tools such as `mkvextract`, `mkvinfo`, and `ffmpeg`.
* Outputs an `.srt` file with the same name as the input file, e.g., `foo.mkv` → `foo.srt`.
* Translates to Brazilian Portuguese.

## Stack

Rust on Ubuntu.

## Development

Keep everything simple. Less code is better.

Git (and `git blame`) is your friend.

In the `tasks/` folder, give your task a number and fill the template:

```
# Task number
...
# What client asked
...
# Technical solution
...
# What changed
...
# Notes
```

Reference the task in your commit. When adding code, if in doubt, look at `git blame` and read the task.

Break responsibilities into different functions and modules.

### Public API focus (Rust)

Prefer clear module and type boundaries. A type should be usable by reading the Rustdoc and looking at the public functions. The best type is the one you don’t need to open to understand.

* Use `struct` + `impl` for concrete types.
* Use `trait` for capabilities/abstractions.
* Use modules (`mod`) and crates to group domains.
* Keep visibility explicit with `pub`, `pub(crate)`, or `pub(super)`.

Move public functions to the top of the `impl` block or module. The more visible and widely used, the higher they should appear in the file.

### Folder & module layout

Use folders to separate code by **domains** first, then by **layers**.

* Higher-level (more depended-on) modules live higher in the tree.
  Lower-level (more independent) modules live deeper.

Example rule: if `B` depends on `A` and `C` depends on `A`, then `A` should be in the parent folder and `B` and `C` in child folders.

Organize so anyone can find code related to a feature by scanning the folder structure—folders reflect app functionality.

Prefer a workspace if it helps isolation:

```
subtitle-translator/          # workspace root
  crates/
    core/                     # domain: core logic (parsing, timing, translation pipeline)
      src/
        lib.rs
        srt/
          mod.rs              // SRT parsing/formatting
        translate/
          mod.rs              // translation orchestration (traits for providers)
        video/
          mod.rs              // subtitle extraction commands
    cli/                      # binary
      src/
        main.rs
    infra/                    # adapters: logging, config, sys processes
      src/
        logging.rs
        process.rs
  tasks/
    0001-something.md
```

### File headers & comments

At the top of each file/module, add a short doc comment explaining the purpose:

```rust
//! This module is responsible for SRT parsing and integrity checks.
//! It exposes minimal helpers to read/write SRT blocks while preserving timing.
```

Use clear variable names. Write as little code as possible—simple is better.

Before **every function**, add a human-readable comment describing the function, even if redundant. Anyone should be able to recreate the function from the comment. Speak as if explaining to a junior dev.

Examples (feel free to vary):

* “In this function we …”
* “This type is responsible for …”
* “This delegates … to …”
* “This function should … and it does it by …”
* “The way this works is …; it’s needed because …; so we return …”
* “Here we isolate the logic for … so we don’t have it in …”
* “Pay attention: the behavior here is … because we need to avoid …”

Avoid useless comments that repeat the code. Prefer contextual explanations and gotchas.

### Logging

Always add logs. Always add a **trace** log at the start of a function.

Log levels intent:

* **info**: Use sparingly, only for important events.
* **warn**: Unwanted or unexpected state; explain why.
* **error**: Unwanted or unexpected state with consequences; explain both.
* **fatal**: Use when the application cannot continue (typically logged right before exit/propagation at top level).
* **trace**: Use a lot—like comments. Narrative of the flow, e.g.,
  `extract_subtitles(path): starting; probing container`,
  `translate_line(idx=42): after batching, sending to provider`.
* **debug**: Use a lot for values, e.g.,
  `Mapping SrtBlock { start, end } to provider input`,
  `Sending 4 lines to translation`.

Use a common logging engine. Recommended: [`tracing`](https://crates.io/crates/tracing) with `tracing-subscriber`:

* Configure it to include module/target so it’s easy to filter.
* Prefer structured fields: `tracing::info!(file=%path.display(), "extracted")`.

### Testing

Always add tests—both happy path and error cases. Ask yourself “what could go wrong?” then add a test to ensure good behavior.

Two styles:

* **Technical**: “This function returns the color corresponding to code XYZ.”
* **Real scenario** (recommended for features): Describe the case in comments, e.g.,
  “User opens the video and expects no extra line breaks; only the ones typed should appear in output.”

Update READMEs to reflect new functionality and build changes!

Basic example:

```rust
/// This test ensures the SRT parser preserves timecodes exactly (happy path).
#[test]
fn parses_and_writes_back_without_time_drift() {
    // ...
}
```

### Code style (Rust)

* Keep lines narrow.
* Prefer early returns with `if`/`if let`/`matches!` at function start instead of deep `else` nesting.

Instead of:

```rust
if let Some(x) = opt {
    if x.special() {
        do_the_thing();
    } else {
        log::info!("not special");
        return;
    }
} else {
    log::info!("none");
    return;
}
```

Prefer:

```rust
if opt.is_none() {
    tracing::info!("none");
    return;
}
let x = opt.unwrap();
if !x.special() {
    tracing::info!("not special");
    return;
}
do_the_thing();
```

* Extract variables with good names and types:

```rust
let current_users_count: u32 = 5;
store(current_users_count);
```

* Prefer iterators where ergonomic; avoid deeply nested loops/conditionals. Flatten with early returns or by extracting helper functions.
* Keep “boilerplate” chained calls visually collapsed so the important bits stand out:

```rust
foo.builder().unwrap().value()
    .set_name("foobar")
    .set_age(50)
.build();
```

* Prefer multiple `map` steps over long closures:

```rust
let bazes = foos.iter()
    .map(Foo::bar)
    .map(Bar::baz)
    .collect::<Vec<_>>();
```

* Rust formatting: use `rustfmt`. Parameters should be formatted as a block when long:

```rust
pub fn foo(
    bar: &str,
    baz: i32,
    qux: f32,
) -> Result<()> {
    // ...
}
```

### Documentation (Rustdoc)

* Item docs: `/// Explains what this function/type does and why.`
* Module/file docs: `//! Explains what this module is responsible for.`
* Show examples in docs where useful; they’re compiled as doctests.

### Error handling

* Use `Result<T, E>` throughout.
* Prefer `thiserror` for domain error types and `anyhow` for application boundaries/CLI.
* Convert external errors with `?` and meaningful context (`with_context` from `anyhow`).

### Process execution

For `mkvextract`, `mkvinfo`, and `ffmpeg`, prefer a small wrapper that:

* Logs the command and sanitized args at `debug`.
* Captures stdout/stderr.
* Maps exit codes to clear errors.

Example signature:

```rust
/// Runs an external command and returns stdout as String.
/// We use this in video/subtitle extraction to keep process handling consistent.
pub fn run_command(
    program: &str,
    args: &[&str],
) -> Result<String> { /* ... */ }
```

### Translation provider

Define a trait for translation so we can swap backends or mock in tests:

```rust
/// Translates a batch of lines with optional context (e.g., video title).
pub trait Translator {
    /// Translate `lines` to the target locale (e.g., "pt-BR"), preserving line boundaries.
    fn translate_batch(
        &self,
        title_context: Option<&str>,
        lines: &[String],
        target_locale: &str,
    ) -> Result<Vec<String>>;
}
```

Provide an OpenAI-backed implementation behind a feature flag if desired.

### SRT handling

Keep a minimal SRT model and integrity checks:

```rust
/// Represents a single SRT block (index, time range, text lines).
pub struct SrtBlock {
    pub index: u32,
    pub start_ms: u64,
    pub end_ms: u64,
    pub text: Vec<String>,
}

/// Parses SRT into blocks and can format back to text.
/// Integrity rule: time ranges are non-overlapping and monotonic.
pub trait SrtCodec {
    fn parse(input: &str) -> Result<Vec<SrtBlock>>;
    fn format(blocks: &[SrtBlock]) -> String;
}
```

When translating, only replace the **text** field(s), keeping indices and timings intact. Use sliding windows of 4 lines before and after to provide context to the translator, without merging blocks.

### CLI

* Provide a single `bin` (`cli`) that:

  * Accepts input file path(s).
  * Extracts English subtitles (via `video` module).
  * Translates to **Brazilian Portuguese** (`pt-BR`).
  * Writes `*.srt` alongside the input file, same stem.
* Add `--dry-run`, `--log-level`, and `--title` overrides.

### Example logging narrative

Start each function with a `trace`:

```rust
/// Extract subtitles from the given path using mkv tools or ffmpeg as fallback.
pub fn extract_english_subtitles(path: &Path) -> Result<PathBuf> {
    tracing::trace!(
        "extract_english_subtitles(path={}): probing container and available tracks",
        path.display()
    );
    // ...
    Ok(out_path)
}
```

Use `info` only for big milestones, e.g., “Translation complete”, “Wrote output”.

### Tests & fixtures

* Add fixtures under `tests/fixtures/` for small SRT samples.
* For process wrappers, use dependency inversion to avoid running external tools in unit tests; integration tests can use a tiny sample file and stub binaries when possible.


## Quick Rust snippets (adapted examples)

**Parameter formatting**

```rust
pub fn process_subtitles(
    input: &Path,
    target_locale: &str,
    title_context: Option<&str>,
) -> Result<PathBuf> {
    // ...
}
```

**Good variable extraction**

```rust
let current_users_count: u32 = 5;
store(current_users_count);
```

**Iterators with multiple maps**

```rust
let translated = blocks.iter()
    .map(|b| b.text.join("\n"))
    .map(|t| t.trim().to_string())
    .collect::<Vec<_>>();
```

**Early returns**

```rust
if blocks.is_empty() {
    tracing::info!("no subtitles found; nothing to do");
    return Ok(output_path);
}
```

---

Keep it small, readable, and well-logged. Add tests for everything you change. Update READMEs whenever behavior or build steps change.