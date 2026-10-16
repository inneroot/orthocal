# orthocal (Rust)

A Rust port of the [orthocal](https://git.sr.ht/~devgioele/orthocal) Orthodox
liturgical calendar CLI (Go → Rust).

`orthocal` prints the Orthodox liturgical calendar — commemoration feasts,
fasting rules, and daily Scripture readings — for a given day, month, or
year, according to the Julian calendar (the default "no reform" mode).

## Usage

```
orthocal [OPTIONS] [[[DAY] MONTH] YEAR]
```

Date argument forms (all Julian dates, year 33 AD to 9999 AD):

| Invocation       | Meaning                          |
|------------------|----------------------------------|
| `2 10 2023`      | that single day                  |
| `10 2023`        | the whole month                  |
| `2023`           | the whole year                   |
| *(no arguments)* | today (local date, converted)    |

Options:

- `--lang <code>` — language: `en` (default) or `it`. Unsupported codes fall
  back to English.
- `--json` — machine-readable output (Go `encoding/json` compatible).
- `--no-readings` — omit Scripture readings.
- `-g` / `--reform proleptic` — interpret input dates as Gregorian
  (proleptic Gregorian reform).
- `--reform <day>,<month>,<year>` — interpret dates before the given Julian
  date as Julian and dates after it as Gregorian (e.g. `15,10,1582`).
- `-v` — print the version.
- `-h` — print usage (exit code 64, like the Go original).

Examples:

```
$ orthocal 2 10 2023
| Tuesday, October 2, 2023 (Julian)
| Tuesday, October 15, 2023 (Gregorian)
| 
| No fasting
| 
| Sts. Cosmas and Damian, physicians of Asia Minor (287 AD)
...

$ orthocal 7 325        # the Council of Nicaea
$ orthocal --lang it 25 12 2025
$ orthocal --json 1970 | jq '.[0]'
```

## Build & test

```
cargo build --release
cargo test
```

The only runtime dependency is `chrono` (local "today" date). All calendar
data (CSV and Fluent assets) is embedded at compile time via `include_str!`.

## Project layout

| File              | Go original                     |
|-------------------|---------------------------------|
| `src/main.rs`     | `cmd/cli` (main + calendarcmd)  |
| `src/cal.rs`      | `pkg/cal`                       |
| `src/paschalion.rs` | `pkg/paschalion`              |
| `src/litcal.rs`   | `pkg/litcal` (commemorations, fasting rules, readings, citations) |
| `src/pdqsort.rs`  | Go `slices.SortFunc` (std-lib port, see below) |
| `src/conv.rs`     | `internal/conv`                 |
| `src/i18n.rs`     | `internal/i18n` (Fluent)        |
| `src/i18ndom.rs`  | `internal/i18ndom`              |
| `src/csvp.rs`     | `internal/csv`                  |
| `src/gjson.rs`    | `encoding/json` marshaling of `[]LiturgicalDay` |
| `src/pretty.rs`   | `cmd/cli/calendarcmd` pretty printer (with `fatih/color` behavior) |
| `src/mmath.rs`    | `pkg/mmath`                     |
| `assets/`         | `src/assets` (CSV + .flt + version) |

The Fluent files only use single-line `key = value` messages with `{$var}`
placeholders, so `i18n.rs` parses them with a small key-value parser instead
of a full Fluent implementation.

## What was ported

- Full calendar logic: Pascha computation, movable/fixed feasts, forefeasts
  and afterfeasts, fasting rules, Scripture readings and citation strings.
- All four date argument forms, both languages, `--json`, `--no-readings`,
  `-v`, `-h`, and the optional Gregorian reform modes (`-g`, `--reform`).
- Byte-compatible output: same formatting, same ANSI color codes and the
  same color-enabling conditions as the Go CLI (terminal stdout, no
  `NO_COLOR`, `TERM != dumb`), same JSON field names/order, same JSON string
  escaping (including Go's HTML escaping of `<`, `>`, `&`), same error
  messages, exit codes (64 for usage, 2 for flag errors, 1 for bad dates),
  and stdout/stderr stream separation.
- `src/pdqsort.rs` is a verbatim port of Go's `slices.SortFunc`
  (pattern-defeating quicksort) so that the order of equal-level
  commemorations matches the Go binary exactly (Go's sort is not stable;
  this keeps the tie-breaking identical).
- Unit tests ported from the Go test files: `paschalion_test.go` (known
  Pascha dates, three independent algorithms in equivalence for years
  1–9999, Great Indiction, Golden Number), `cal_test.go` (date
  construction/validation, Julian↔Gregorian conversion, weekday,
  nth/closest weekday, distance, shifts, day/year ranges, Byzantine year),
  and `cite_test.go` (citation range rendering), plus tests for the new
  i18n/JSON/pdqsort code.

## What was dropped

- `cli2`/`cli3`/`cli4` helper CLIs and the website generator
  (`cmd/generate`), web templates and static assets.
- The generated man page; `-h` prints the same two-line synopsis the Go
  binary prints.
- `pkg/paths` (used only by the website generator).

## Known differences from the Go reference

1. **Same-level reading order is nondeterministic in the Go reference.**
   `projectReadings` iterates a Go map, so the order of reading groups with
   equal level varies from run to run. This port is deterministic
   (first-seen CSV order before the shared pdqsort), so any single Go run may
   differ in the order of equal-level readings; the Rust output is always
   one of the valid Go orderings.
2. **Same-book readings are merged in this port.** The Go reference builds
   its reading group key with `fmt.Sprintf("%v", *int)`, which prints the
   pointer *address*, so every reading ends up in its own group and is
   printed on its own line. This port groups by the actual values, so e.g.
   the four Luke readings of 24 June are printed as one citation
   `Luke 1:1-15, 57-68, 76, 80` instead of four lines. The set of readings
   is identical; this affects a few days per year (e.g. 3 days in 1970).
3. **`--lang` fallback.** The Go matcher falls back to a nondeterministic
   default for unknown codes (Go map ordering); this port falls back to
   English deterministically. The observable deterministic behaviors match,
   including the `x/text` quirk where `it-IT` does *not* match Italian.
4. **`--reform` with fewer than three numbers** panics (runtime crash) in the
   Go reference; this port returns `invalid date format` instead.
5. `cal.ShiftMonth`/`ShiftYear` preserve the Go original's use of Julian
   month lengths (documented in code); they are not used by the CLI.

## Validation

```
cd /home/inner/dev/orthocal/src && go build -o /tmp/orthocal-go ./cmd/cli   # reference
cargo build --release && cargo test   # 37 tests, all pass
```

Outputs were diffed for a battery of cases (single day, month, full year
1970 and 325, `--lang it`, `--json`, `--no-readings`, reform modes, today,
error cases, `-v`, `-h`):

- 21 cases byte-identical, including full year 1970 with `--no-readings`,
  the 36-commemoration day 1 3 2023 (verifying the pdqsort port), all
  `--no-readings` cases, today, error messages/exit codes, and both reform
  modes.
- The remaining cases are identical except for the two reading-related
  differences above (same-level reading order, and same-book citation
  merging), verified by a normalized comparison that flattens reading
  groups and sorts citation segments per day: all equal.
- JSON output was compared after flattening each day's `Readings` array to
  its component readings: identical, including Go's `\u0026` HTML escaping,
  `null` for empty slices, and field order.
