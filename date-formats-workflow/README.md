# Date Formats Workflow

Date Formats Alfred Workflow for quick DateTime formatting and locale adjustment.

Requirements
-------------
N/A

Installation
-------------
1. Download date-formats-workflow.alfredworkflow from the repo's [releases](https://github.com/rust-playground/alfred-workflows-rs/releases) section
2. Install in Alfred (double-click)

Usage
------
By default all date & times are displayed in UTC, however, this is overridable by specifying the
timezone with the ending argument `-t,--tz [timezone]`; examples shown below.

- `df now` displays various date formats for the current date & time.
- `df [date & time string]` the date & time string provided is parsed and date formats for this time displayed. Supported formats include:
  - ISO8601
  - RFC3339
  - RFC2822
  - yyyy-mm-dd
  - yyyy-mm-dd hh:mm:ss
  - yyyy-mm-dd hh:mm:ss tz
  - Fri Nov 28 12:00:09 2014
  - UNIX Timestamp - seconds, millisecond and nanoseconds
- Adding `-t [timezone]` or `--tz [timezone]` to the command will convert the dates to the provided timezone, eg. `df now --tz PST` will display the current time in PST('America/Vancouver' in this case). 
