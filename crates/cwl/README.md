# CWL
This crate provides support for working with Common Workflow Language (CWL) files.
It includes modules for handling CommandLineTools, Workflows, ExpressionTools, and their associated metadata.
## Modules
- `clt`: Handles CWL CommandLineTools.
- `et`: Handles CWL ExpressionTools.
- `wf`: Handles CWL Workflows.
- `inputs`: Defines and deserializes input parameters.
- `outputs`: Defines and deserializes output parameters.
- `requirements`: Handles CWL requirements and hints.
- `types`: Provides CWL-specific types.
- `format`: Utilities for formatting CWL files.
- `deserialize`: Shared deserialization utilities.
## Examples
```rust
use cwl;
let clt = cwl::load_tool("example-tool.cwl")?;
let et = cwl::load_expression_tool("example-expr.cwl")?;
let wf = cwl::load_workflow("example-wf.cwl")?;
```
## Usage
To use this crate, add it as a dependency in your `Cargo.toml`:
```toml
[dependencies]
cwl = "*"
```