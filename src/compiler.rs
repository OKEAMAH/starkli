use std::{
    fmt::Display,
    path::{Path, PathBuf},
    process::Command,
    str::FromStr,
};

use anyhow::Result;
use cairo_starknet_2_6_4::{
    casm_contract_class::CasmContractClass as Cairo264CasmClass,
    contract_class::ContractClass as Cairo264Class,
};
use cairo_starknet_2_7_0::{
    casm_contract_class::CasmContractClass as Cairo270CasmClass,
    contract_class::ContractClass as Cairo270Class,
};
use clap::{builder::PossibleValue, ValueEnum};
use starknet::core::types::{
    contract::{CompiledClass, SierraClass},
    Felt,
};

const MAX_BYTECODE_SIZE: usize = 180000;

#[derive(Debug)]
pub struct BuiltInCompiler {
    version: CompilerVersion,
}

#[derive(Debug)]
pub struct CompilerBinary {
    path: PathBuf,
}

// TODO: separate known compiler versions with linked versions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompilerVersion {
    V2_6_4,
    V2_7_0,
}

impl BuiltInCompiler {
    pub fn version(&self) -> CompilerVersion {
        self.version
    }

    pub fn compile(&self, class: &SierraClass) -> Result<Felt> {
        // We do this because the Sierra doesn't need ABI anyways. Feeding it with the ABI could
        // actually cause unnecessary deserialization errors due to ABI structure changes between
        // compiler versions.
        let mut class = class.clone();
        class.abi.clear();

        let sierra_class_json = serde_json::to_string(&class)?;

        let casm_class_json = match self.version {
            CompilerVersion::V2_6_4 => {
                // TODO: directly convert type without going through JSON
                let contract_class: Cairo264Class = serde_json::from_str(&sierra_class_json)?;

                // TODO: implement the `validate_compatible_sierra_version` call

                let casm_contract = Cairo264CasmClass::from_contract_class(
                    contract_class,
                    false,
                    MAX_BYTECODE_SIZE,
                )?;

                serde_json::to_string(&casm_contract)?
            }
            CompilerVersion::V2_7_0 => {
                // TODO: directly convert type without going through JSON
                let contract_class: Cairo270Class = serde_json::from_str(&sierra_class_json)?;

                // TODO: implement the `validate_compatible_sierra_version` call

                let casm_contract = Cairo270CasmClass::from_contract_class(
                    contract_class,
                    false,
                    MAX_BYTECODE_SIZE,
                )?;

                serde_json::to_string(&casm_contract)?
            }
        };

        // TODO: directly convert type without going through JSON
        let casm_class = serde_json::from_str::<CompiledClass>(&casm_class_json)?;

        let casm_class_hash = casm_class.class_hash()?;

        Ok(casm_class_hash)
    }
}

impl CompilerBinary {
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn compile(&self, class: &SierraClass) -> Result<Felt> {
        // We do this because the Sierra doesn't need ABI anyways. Feeding it with the ABI could
        // actually cause unnecessary deserialization errors due to ABI structure changes between
        // compiler versions.
        let mut class = class.clone();
        class.abi.clear();

        let mut input_file = tempfile::NamedTempFile::new()?;
        serde_json::to_writer(&mut input_file, &class)?;

        let process_output = Command::new(&self.path)
            .arg(
                input_file
                    .path()
                    .to_str()
                    .ok_or_else(|| anyhow::anyhow!("invalid temp file path"))?,
            )
            .output()?;

        if !process_output.status.success() {
            anyhow::bail!(
                "Sierra compiler process failed with exit code: {}",
                process_output.status
            );
        }

        let casm_class_json = String::from_utf8(process_output.stdout)?;

        let casm_class = serde_json::from_str::<CompiledClass>(&casm_class_json)?;

        let casm_class_hash = casm_class.class_hash()?;

        Ok(casm_class_hash)
    }
}

impl Default for CompilerVersion {
    fn default() -> Self {
        Self::V2_6_4
    }
}

impl ValueEnum for CompilerVersion {
    fn value_variants<'a>() -> &'a [Self] {
        &[Self::V2_6_4, Self::V2_7_0]
    }

    fn to_possible_value(&self) -> Option<PossibleValue> {
        match self {
            Self::V2_6_4 => Some(PossibleValue::new("2.6.4").alias("v2.6.4")),
            Self::V2_7_0 => Some(PossibleValue::new("2.7.0").alias("v2.7.0")),
        }
    }
}

impl FromStr for CompilerVersion {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "2.6.4" | "v2.6.4" => Ok(Self::V2_6_4),
            "2.7.0" | "v2.7.0" => Ok(Self::V2_7_0),
            _ => Err(anyhow::anyhow!("unknown version: {}", s)),
        }
    }
}

impl Display for CompilerVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompilerVersion::V2_6_4 => write!(f, "2.6.4"),
            CompilerVersion::V2_7_0 => write!(f, "2.7.0"),
        }
    }
}

impl From<CompilerVersion> for BuiltInCompiler {
    fn from(value: CompilerVersion) -> Self {
        Self { version: value }
    }
}

impl From<PathBuf> for CompilerBinary {
    fn from(value: PathBuf) -> Self {
        Self { path: value }
    }
}
