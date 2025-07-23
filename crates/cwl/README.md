# CommonWL
This crate provides support for working with [Common Workflow Language](https://www.commonwl.org/v1.2/) (CWL) files.
It includes modules for handling [CommandLineTools](https://www.commonwl.org/v1.2/CommandLineTool.html), [Workflows](https://www.commonwl.org/v1.2/Workflow.html), [ExpressionTools](https://www.commonwl.org/v1.2/Workflow.html#ExpressionTool), and their associated metadata.

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

## Example
```rust
use commonwl;
let clt = commonwl::load_tool("example-tool.cwl")?;
let et = commonwl::load_expression_tool("example-expr.cwl")?;
let wf = commonwl::load_workflow("example-wf.cwl")?;
```

## Installation
> [!CAUTION]
> Crate has not been submitted to crates.io, yet!

Run the following Cargo command in your project directory:
```
cargo add cwl
```
Or add the following line to your Cargo.toml:
```toml
[dependencies]
commonwl = "0.3.0"
```