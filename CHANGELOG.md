# Unreleased
## 🚀 Features
- added NetworkAccess Requirment via `--net/--enable-network` in `s4n tool create` #104
- Support InlineTools in Workflow step #111
- Support Files and Directories as `InitialWorkDirRequirement` (in additon to Dirents)

## 🐛 Bugfixes
- fixed a bug with Dockerfile path resolution
- handle NetworkAccess Requirment in runner
- ramping up runner conformance from 160/378 to 177/378

# v0.5.2
## 🐛 Bugfixes
- fixed bugs with the `tool create -i` argument

# v0.5.1
## 🐛 Bugfixes
- fixed serialisation error

# v0.5.0
## 🚀 Features
- Support automatic downloading if files are given with `http://` or `https://` protocol
- Detect URLs as files if given with `http://` or `https://` protocol
- Support Detection of Arrays as inputs in `tool create` #100
- Support ExpressionLibs

## 🐛 Bugfixes
- correctly support `DockerRequirement.DockerOutputDirectory`#96
- made `CommandOutputBinding.glob` Optional #99
- ramping up runner conformance from 131/378 to 160/378

# v0.4.0
## 🚀 Features
- Added `--no-defaults` flag to tool create which can be handy when using passwords as inputs

## 🐛 Bugfixes
- fixed critical error in tool create where `outputEval` was set, even when null
- 🏃CWL Runner
    - Fixed some bugs in CWL Runner ramping up its conformance from 126/378 to 131/378
    - Improved Array support in Runner
    - Support cwl.output.json handling in Runner
    - Support globs in Runner
- Rewrite input ids if "bad words" are found (e.g. sql connection strings)

# v0.3.0
## 🚀 Features
- Added Containerization Support (Docker & Podman) for `s4n execute local`
- Support CWL ExpressionTools

## 🐛 Bugfixes
- Fixed some bugs in CWL Runner ramping up its conformance from 90/378 to 126/378

## 👀 Miscellaneous Tasks
- Removed Nightly Builds CI Workflow

# v0.2.0
## 🚀 Features
- Allowed handling of nullable and array CWLTypes using `File?` or `File[]` notation
- Added `s4n execute make-template ./path/to.cwl` to create job templates #75
- Added support for the direct execution of files #79
- Allow Directories as output

## 🐛 Bugfixes
- Fixed setting correct InitialWorkDirRequirement when `-i` is used in `s4n tool create` #69
- Fixed handling of json-Data #60
- Fixed unreported Bug, where CWL CommandLineTool Output was ignored if not of type File, Directory, stdout, stderr or string. 781d20e
- Fixed Command fail because of invalid git user config - prompts user if missing #78
- Fixed cleanup if init fails #77
- Fixed Files in subfolders can not be created in s4n tool create #88
- Fixed Do not check for uncommited changes if --no-run #89

## 🚜 Refactor
- Moved Runner into separate crate (Refactor)

## 👀 Miscellaneous Tasks
- Added Tests for all Documentation examples #76
- Added CWL Conformance Tests to CI Workflow
- Added more integration tests

# v0.1.0
Initial Release
