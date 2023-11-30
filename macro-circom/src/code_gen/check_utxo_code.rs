use std::collections::HashSet;

use heck::ToUpperCamelCase;

// TODO: add check that a Utxo type exists for every Utxo
use crate::code_gen::utxo_type::UtxoType;
use crate::errors::MacroCircomError;

#[derive(Clone, Debug, PartialEq, Default)]
pub struct UtxoChecks {
    pub amount_sol: Option<(Comparator, String)>,
    pub amount_spl: Option<(Comparator, String)>,
    pub asset_spl: Option<(Comparator, String)>,
    pub blinding: Option<(Comparator, String)>,
    pub public_key: Option<(Comparator, String)>,
    /// Mark a utxo as program/app utxo, are for native utxos default to 0 and are checked by default.
    pub utxo_data_hash: Option<(Comparator, String)>,
    pub psp_owner: Option<(Comparator, String)>,
    /// not supported yet
    pub tx_version: Option<(Comparator, String)>,
    pub pool_type: Option<(Comparator, String)>,
}
pub trait FieldSetter {
    fn set_field(
        &mut self,
        field_name: &str,
        comparator: Comparator,
        value: String,
    ) -> Result<(), MacroCircomError>;
}

impl FieldSetter for UtxoChecks {
    fn set_field(
        &mut self,
        field_name: &str,
        comparator: Comparator,
        value: String,
    ) -> Result<(), MacroCircomError> {
        match field_name {
            "amountSol" => self.amount_sol = Some((comparator, value)),
            "amountSpl" => self.amount_spl = Some((comparator, value)),
            "assetSpl" => self.asset_spl = Some((comparator, value)),
            "utxoDataHash" => self.utxo_data_hash = Some((comparator, value)),
            "pspOwner" => self.psp_owner = Some((comparator, value)),
            "txVersion" => self.tx_version = Some((comparator, value)),
            "poolType" => self.pool_type = Some((comparator, value)),
            "blinding" => self.blinding = Some((comparator, value)),
            "publicKey" => self.public_key = Some((comparator, value)),
            _ => return Err(MacroCircomError::UnknowField(field_name.to_string())),
        }
        Ok(())
    }
}

impl UtxoChecks {
    pub fn from_data(data: Vec<(String, Comparator, String)>) -> Result<Self, MacroCircomError> {
        let mut result = UtxoChecks::default();
        for (field_name, comparator, value) in data {
            result.set_field(&field_name, comparator, value)?;
        }
        Ok(result)
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Utxo {
    pub is_declared: bool,
    pub is_checked: bool,
    pub check_code: String,
    pub declare_code: String,
    pub name: String,
    pub type_struct: Option<UtxoType>,
    pub type_name: String,
    pub is_in_utxo: bool,
    pub instruction_name: Option<String>,
    pub no_utxos: String,
    pub checks: Option<UtxoChecks>,
    pub utxo_data_checks: Option<Vec<(String, Comparator, String)>>,
}

pub fn instantiate_utxo(
    is_in_utxo: bool,
    name: String,
    fields: (
        String,
        Option<String>,
        String,
        Option<UtxoChecks>,
        Option<Vec<(String, Comparator, String)>>,
    ),
) -> Utxo {
    Utxo {
        is_declared: false,
        is_checked: false,
        check_code: String::new(),
        declare_code: String::new(),
        name: name.to_string(),
        type_struct: None,
        type_name: fields.0.to_string(),
        is_in_utxo,
        instruction_name: fields.1,
        no_utxos: fields.2.to_string(),
        checks: fields.3,
        utxo_data_checks: fields.4,
    }
}

#[derive(PartialEq, Debug, Clone)]
pub enum Comparator {
    Equal,
    NotEqual,
    GreaterThan,
    LessThan,
    GreaterEqualThan,
    LessEqualThan,
}

impl Comparator {
    pub fn as_str(&self) -> &str {
        match *self {
            Comparator::Equal => "==",
            Comparator::NotEqual => "!=",
            Comparator::GreaterThan => ">",
            Comparator::LessThan => "<",
            Comparator::GreaterEqualThan => ">=",
            Comparator::LessEqualThan => "<=",
        }
    }
}

impl Utxo {
    fn generate_declare_code(&mut self) -> Result<(), MacroCircomError> {
        let template = r#"
        signal input is{{is_In}}AppUtxo{{UtxoName}}[{{is_ins}}];
        var sumIs{{is_In}}AppUtxo{{UtxoName}} = 0;
        for (var i= 0; i < {{is_ins}}; i++) {
            (1 - is{{is_In}}AppUtxo{{UtxoName}}[i]) * is{{is_In}}AppUtxo{{UtxoName}}[i] === 0;
            sumIs{{is_In}}AppUtxo{{UtxoName}} += is{{is_In}}AppUtxo{{UtxoName}}[i];
        }
        sumIs{{is_In}}AppUtxo{{UtxoName}} === 1 * {{instruction}};
        {{#if len_utxo_data_non_zero}}
        {{#with this}}{{#each utxoData}}
        signal input {{../../utxoName}}{{this.Input}};
        {{/each}}{{/with}}

        component utxoDataHasher{{UtxoName}} = Poseidon({{utxo_data_length}});

        {{#with this}}{{#each utxoData}}
        utxoDataHasher{{../../UtxoName}}.inputs[{{@index}}] <== {{../../utxoName}}{{this.Input}};
        {{/each}}{{/with}}

        component checkInstructionHash{{UtxoName}}[{{is_ins}}];
        for (var inUtxoIndex = 0; inUtxoIndex < {{is_ins}}; inUtxoIndex++) {
            checkInstructionHash{{UtxoName}}[inUtxoIndex] = ForceEqualIfEnabled();
            checkInstructionHash{{UtxoName}}[inUtxoIndex].in[0] <== {{is_in}}AppDataHash[inUtxoIndex];
            checkInstructionHash{{UtxoName}}[inUtxoIndex].in[1] <== utxoDataHasher{{UtxoName}}.out;
            checkInstructionHash{{UtxoName}}[inUtxoIndex].enabled <== is{{is_In}}AppUtxo{{UtxoName}}[inUtxoIndex];
        }
        {{/if}}
        component {{utxoName}} = {{this.utxoType}}();

        {{#with this}}{{#each utxoData}}
        {{../../utxoName}}.{{this.input}}In <== {{../../utxoName}}{{this.Input}};
        {{/each}}{{/with}}
        {{#with this}}{{#each nativeUtxoFields}}
        signal input {{../../utxoName}}{{this.InputField}};
        {{../../utxoName}}.{{this.inputField}}In <== {{../../utxoName}}{{this.InputField}};
        {{/each}}{{/with}}
        {{../../utxoName}}.utxoDataHashIn <== {{isAppUtxo}};

        component {{../../utxoName}}AmountHasher = Poseidon(2);
        {{../../utxoName}}AmountHasher.inputs[0] <== 0;
        {{../../utxoName}}AmountHasher.inputs[1] <== {{../../utxoName}}.assetSpl;

        component {{../../utxoName}}AssetHasher = Poseidon(2);
        {{../../utxoName}}AssetHasher.inputs[0] <== {{../../utxoName}}.amountSol;
        {{../../utxoName}}AssetHasher.inputs[1] <== {{../../utxoName}}.amountSpl;

        component {{../../utxoName}}UtxoCheckHasher = Poseidon(8);
        {{../../utxoName}}UtxoCheckHasher.inputs[0] <== 0; // TxVersion
        {{../../utxoName}}UtxoCheckHasher.inputs[1] <== {{../../utxoName}}AmountHasher.out;
        {{../../utxoName}}UtxoCheckHasher.inputs[2] <== {{../../utxoName}}AssetHasher.out;
        {{../../utxoName}}UtxoCheckHasher.inputs[3] <== {{../../utxoName}}.blinding;
        {{../../utxoName}}UtxoCheckHasher.inputs[4] <== {{../../utxoName}}AssetHasher.out;
        {{../../utxoName}}UtxoCheckHasher.inputs[5] <== {{../../utxoName}}.utxoDataHash;
        {{../../utxoName}}UtxoCheckHasher.inputs[6] <== 0;
        {{../../utxoName}}UtxoCheckHasher.inputs[7] <== {{../../utxoName}}.pspOwner;
"#;
        // TODO: make direct part of transaction hash
        // - put check at the end of the transaction
        // - don't compute utxo hashes of non-declared utxos just pass in the commitments
        let mut all_utxo_data = Vec::<handlebars::JsonValue>::new();
        for utxo_data_field in &self.type_struct.as_ref().unwrap().fields {
            all_utxo_data.push(serde_json::json!({
                "component": format!("UtxoData{}", utxo_data_field.clone().to_upper_camel_case()),
                "input": utxo_data_field,
                "Input": utxo_data_field.to_upper_camel_case(),
            }));
        }

        // This handles the case that we want to check a utxo that is not a program utxo the utxo data is zero
        let len_utxo_data = if self.type_struct.as_ref().is_some() {
            self.type_struct.as_ref().unwrap().fields.len()
        } else {
            0
        };

        let native_utxo_fields = vec![
            "publicKey",
            "blinding",
            "pspOwner",
            "amountSol",
            "amountSpl",
            "assetSpl",
            "txVersion",
            "poolType",
        ];
        let mut native_utxo_fields_vec = Vec::<handlebars::JsonValue>::new();
        for utxo_field in &native_utxo_fields {
            native_utxo_fields_vec.push(serde_json::json!({
                "inputField": utxo_field,
                "InputField": utxo_field.to_upper_camel_case(),
            }));
        }

        let instruction = match &self.instruction_name {
            Some(intruction) => intruction.clone(),
            None => String::from("1"),
        };
        let data = serde_json::json!({
            "is_ins": if self.is_in_utxo { "nIns" } else { "nOuts" },
            "is_In": if self.is_in_utxo { "In" } else { "Out" },
            "is_in": if self.is_in_utxo { "in" } else { "out" },
            "utxoName": self.name,
            "UtxoName": self.name.to_upper_camel_case(),
            "utxoType": self.type_name.to_upper_camel_case(),
            "utxoData": all_utxo_data,
            "nativeUtxoFields": native_utxo_fields_vec,
            "utxo_data_length": len_utxo_data,
            "len_utxo_data_non_zero": len_utxo_data > 0,
            "instruction":instruction,
            "isAppUtxo": if self.type_name != "native" {format!("utxoDataHasher{}.out", self.name.to_upper_camel_case())} else {String::from("0")}
        });
        let handlebars = handlebars::Handlebars::new();

        match handlebars.render_template(template, &data) {
            Ok(res) => {
                self.declare_code = res;
                Ok(())
            }
            Err(err) => Err(MacroCircomError::CodeGenerationFailed(
                format!("declaration code Utxo {}", self.name),
                format!("{}", err),
            )),
        }
    }

    fn generate_check_code(&mut self) -> Result<(), MacroCircomError> {
        let template = r#"
{{#each comparisonsUtxoData}} {{#with this}}
component check{{this.component}}{{../../UtxoName}}[{{../../is_ins}}];
{{/with}}{{/each}}

{{#if comparisons}}
{{#each comparisons}}{{#with this}}
component check{{this.component}}{{../../UtxoName}}[{{../../is_ins}}];
{{/with}}{{/each}}
{{/if}}
for (var i = 0; i < {{is_ins}}; i++) {
{{#if comparisons}}
{{#with this}} {{#each comparisons}}

    check{{is_In}}{{this.component}}{{../../UtxoName}}[i] = ForceEqualIfEnabled();
    check{{is_In}}{{this.component}}{{../../UtxoName}}[i].in[0] <== {{../../is_in}}{{this.hasher}}[i]{{this.input}};
    check{{is_In}}{{this.component}}{{../../UtxoName}}[i].in[1] <== {{this.comparison}};
    check{{is_In}}{{this.component}}{{../../UtxoName}}[i].enabled <== is{{../../is_In}}AppUtxo{{../../UtxoName}}[i] * {{../../instruction}};

{{/each}}{{/with}}
{{/if}}

{{#if comparisonsUtxoData}}
{{#each comparisonsUtxoData}}{{#with this}}

    check{{this.component}}{{../../UtxoName}}[i] = ForceEqualIfEnabled();
    check{{this.component}}{{../../UtxoName}}[i].in[0] <== {{../../utxoName}}.{{this.input}};
    check{{this.component}}{{../../UtxoName}}[i].in[1] <== {{this.comparison}};
    check{{this.component}}{{../../UtxoName}}[i].enabled <== is{{../../is_In}}AppUtxo{{../../UtxoName}}[i] * {{../../instruction}};

{{/with}}{{/each}}
{{/if}}
}

"#;
        let mut comparisons = vec![];
        if self.type_name == "native" {
            comparisons.push(serde_json::json!({
                "component": "AppDataHash",
                "hasher": "AppDataHash",
                "input": "",
                "comparison": "0",
            }));
            comparisons.push(serde_json::json!({
                "component": "PspOwner",
                "hasher": "CommitmentHasher",
                "input": ".inputs[7]",
                "comparison": "0",
            }));
        }

        if self.checks.is_some() {
            if let Some((_, value)) = &self.checks.as_ref().unwrap().amount_sol {
                comparisons.push(serde_json::json!({
                    "component": "AmountSol",
                    "hasher": "AmountsHasher",
                    "input": ".inputs[0]",
                    "comparison": value,
                }));
            }

            if let Some((_, value)) = &self.checks.as_ref().unwrap().amount_spl {
                comparisons.push(serde_json::json!({
                    "component": "AmountSpl",
                    "hasher": "AmountsHasher",
                    "input": ".inputs[1]",
                    "comparison": value,
                }));
            }

            if let Some((_, value)) = &self.checks.as_ref().unwrap().asset_spl {
                comparisons.push(serde_json::json!({
                    "component": "AssetSpl",
                    "hasher": "AssetsHasher",
                    "input": ".inputs[1]",
                    "comparison": value,
                }));
            }

            if let Some((_, value)) = &self.checks.as_ref().unwrap().blinding {
                comparisons.push(serde_json::json!({
                    "component": "Blinding",
                    "hasher": "CommitmentHasher",
                    "input": ".inputs[3]",
                    "comparison": value,
                }));
            }

            if let Some((_, value)) = &self.checks.as_ref().unwrap().public_key {
                comparisons.push(serde_json::json!({
                    "component": "PublicKey",
                    "hasher": "CommitmentHasher",
                    "input": ".inputs[2]",
                    "comparison": value,
                }));
            }

            if let Some((_, value)) = &self.checks.as_ref().unwrap().utxo_data_hash {
                if self.type_name == "native" {
                    panic!("UtxoDataHash == 0 is checked by default for native utxos.")
                }
                comparisons.push(serde_json::json!({
                    "component": "AppDataHash",
                    "hasher": "AppDataHash", // TODO: rename together with zk.js
                    "input": "",
                    "comparison": value,
                }));
            }

            if let Some((_, value)) = &self.checks.as_ref().unwrap().psp_owner {
                if self.type_name == "native" {
                    panic!("pspOwner == 0 is checked by default for native utxos.")
                }
                comparisons.push(serde_json::json!({
                    "component": "PspOwner",
                    "hasher": "CommitmentHasher",
                    "input": ".inputs[7]",
                    "comparison": value,
                }));
            }

            // Tx version and pool type are checked by default, additional checks are not supported yet
            // but implemented for completeness.
            if let Some((_, value)) = &self.checks.as_ref().unwrap().tx_version {
                comparisons.push(serde_json::json!({
                    "component": "TxVersion",
                    "hasher": "CommitmentHasher",
                    "input": ".inputs[0]",
                    "comparison": value,
                }));
                unimplemented!("TxVersion has to be 0 and is checked automatically.");
            }
            if let Some((_, value)) = &self.checks.as_ref().unwrap().pool_type {
                comparisons.push(serde_json::json!({
                    "component": "PoolType",
                    "hasher": "CommitmentHasher",
                    "input": ".inputs[6]",
                    "comparison": value,
                }));
                unimplemented!("Pool type has to be 0 and is checked automatically.");
            }
        }
        let mut comparisons_utxo_data = Vec::<handlebars::JsonValue>::new();
        if let Some(utxo_data_checks) = self.utxo_data_checks.as_ref() {
            for utxo_data_check in utxo_data_checks {
                comparisons_utxo_data.push(serde_json::json!({
                    "component": format!("UtxoData{}", utxo_data_check.0.to_upper_camel_case()),
                    "input": utxo_data_check.0,
                    "comparison": utxo_data_check.2,
                }));
            }
        }

        let handlebars = handlebars::Handlebars::new();

        // This handles the case that we want to check a utxo that is not a program utxo the utxo data is zero
        let len_utxo_data = if self.utxo_data_checks.as_ref().is_some() {
            self.utxo_data_checks.as_ref().unwrap().len()
        } else {
            0
        };
        let instruction = match &self.instruction_name {
            Some(intruction) => intruction.clone(),
            None => String::from("1"),
        };
        let data = serde_json::json!({
            "is_ins": if self.is_in_utxo { "nIns" } else { "nOuts" },
            "is_In": if self.is_in_utxo { "In" } else { "Out" },
            "is_in": if self.is_in_utxo { "in" } else { "out" },
            "utxoName": self.name,
            "UtxoName": self.name.to_upper_camel_case(),
            "utxoType": self.type_name.to_upper_camel_case(),
            "instruction": instruction,
            "comparisons": comparisons,
            "comparisonsUtxoData": comparisons_utxo_data,
            "utxo_data_length": len_utxo_data,
            "len_utxo_data_non_zero": len_utxo_data > 0,
        });
        // let res = handlebars.render_template(template, &data).unwrap();
        match handlebars.render_template(template, &data) {
            Ok(res) => {
                self.check_code = res;
                Ok(())
            }
            Err(err) => Err(MacroCircomError::CodeGenerationFailed(
                format!("Utxo {}", self.name),
                format!("{}", err),
            )),
        }
    }
}

pub fn generate_check_utxo_code(checked_utxo: &mut Vec<Utxo>) -> Result<(), MacroCircomError> {
    check_for_duplicates(checked_utxo)?;
    for utxo in checked_utxo {
        if utxo.no_utxos.parse::<u64>().unwrap() == 0 {
            continue;
        } else if utxo.no_utxos.parse::<u64>().unwrap() > 1 {
            unimplemented!("Multiple utxos not supported yet.");
        }
        utxo.generate_declare_code()?;
        utxo.generate_check_code()?;
    }

    Ok(())
}

fn check_for_duplicates(v: &Vec<Utxo>) -> Result<(), MacroCircomError> {
    let mut seen = HashSet::new();

    for item in v {
        if !seen.insert(&item.name) {
            return Err(MacroCircomError::DuplicateUtxoCheck(item.name.clone()));
        }
    }
    Ok(())
}

pub fn assign_utxo_type(
    utxo_types: &Vec<UtxoType>,
    utxo_checks: &mut Vec<Utxo>,
) -> Result<(), MacroCircomError> {
    for utxo_check in utxo_checks {
        for utxo_type in utxo_types {
            if utxo_check.type_name == utxo_type.name {
                utxo_check.type_struct = Some(utxo_type.clone());
            }
        }
        if utxo_check.type_struct.is_none() {
            return Err(MacroCircomError::UtxoTypeNotFound(
                utxo_check.type_name.clone(),
                utxo_check.name.clone(),
            ));
        }
    }
    Ok(())
}

mod tests_utxo {
    #[allow(unused_imports)]
    use super::*;
    #[allow(unused_imports)]
    use crate::describe_error;
    #[allow(unused_imports)]
    use crate::utils::assert_syn_eq;
    #[allow(unused_imports)]
    use crate::utils::remove_formatting;
    #[allow(unused_imports)]
    use crate::Comparator;

    #[test]
    fn generate_delare_code_test() -> Result<(), MacroCircomError> {
        let checks = UtxoChecks {
            amount_sol: Some((Comparator::Equal, "sth".to_string())),
            amount_spl: None,
            asset_spl: None,
            utxo_data_hash: None,
            blinding: None,
            tx_version: None,
            pool_type: None,
            psp_owner: None,
            public_key: None,
        };

        // Setting up a Utxo instance with mock data
        let mut check_utxo = Utxo {
            is_checked: false,
            is_declared: false,
            declare_code: String::new(),
            check_code: String::new(),
            name: "UtxoName".to_string(),
            type_name: "UtxoType".to_string(),
            type_struct: Some(UtxoType {
                fields: vec!["attribute1".to_string(), "attribute2".to_string()],
                name: "UtxoType".to_string(),
                code: String::new(),
            }),
            is_in_utxo: true,
            instruction_name: Some("instruction".to_string()),
            no_utxos: String::from("1"),
            checks: Some(checks),
            utxo_data_checks: Some(vec![(
                "attribute2".to_string(),
                Comparator::Equal,
                "testComparison".to_string(),
            )]),
        };

        check_utxo.generate_declare_code()?;

        let expected_output = r#"
    signal input isInAppUtxoUtxoName[nIns];
    var sumIsInAppUtxoUtxoName = 0;
    for (var i= 0; i < nIns; i++) {
        (1 - isInAppUtxoUtxoName[i]) * isInAppUtxoUtxoName[i] === 0;
        sumIsInAppUtxoUtxoName += isInAppUtxoUtxoName[i];
    }
    sumIsInAppUtxoUtxoName === 1 * instruction;

    signal input UtxoNameAttribute1;

    signal input UtxoNameAttribute2;


    component utxoDataHasherUtxoName = Poseidon(2);

    utxoDataHasherUtxoName.inputs[0] <== UtxoNameAttribute1;
    utxoDataHasherUtxoName.inputs[1] <== UtxoNameAttribute2;

    component checkInstructionHashUtxoName[nIns];
    for (var inUtxoIndex = 0; inUtxoIndex < nIns; inUtxoIndex++) {
        checkInstructionHashUtxoName[inUtxoIndex] = ForceEqualIfEnabled();
        checkInstructionHashUtxoName[inUtxoIndex].in[0] <== inAppDataHash[inUtxoIndex];
        checkInstructionHashUtxoName[inUtxoIndex].in[1] <== utxoDataHasherUtxoName.out;
        checkInstructionHashUtxoName[inUtxoIndex].enabled <== isInAppUtxoUtxoName[inUtxoIndex];
    }
    component UtxoName = UtxoType();


        UtxoName.attribute1In <== UtxoNameAttribute1;

        UtxoName.attribute2In <== UtxoNameAttribute2;


        signal input UtxoNamePublicKey;
        UtxoName.publicKeyIn <== UtxoNamePublicKey;

        signal input UtxoNameBlinding;
        UtxoName.blindingIn <== UtxoNameBlinding;

        signal input UtxoNamePspOwner;
        UtxoName.pspOwnerIn <== UtxoNamePspOwner;

        signal input UtxoNameAmountSol;
        UtxoName.amountSolIn <== UtxoNameAmountSol;

        signal input UtxoNameAmountSpl;
        UtxoName.amountSplIn <== UtxoNameAmountSpl;

        signal input UtxoNameAssetSpl;
        UtxoName.assetSplIn <== UtxoNameAssetSpl;

        signal input UtxoNameTxVersion;
        UtxoName.txVersionIn <== UtxoNameTxVersion;

        signal input UtxoNamePoolType;
        UtxoName.poolTypeIn <== UtxoNamePoolType;

        UtxoName.utxoDataHashIn <== utxoDataHasherUtxoName.out;

        component UtxoNameAmountHasher = Poseidon(2);
        UtxoNameAmountHasher.inputs[0] <== 0;
        UtxoNameAmountHasher.inputs[1] <== UtxoName.assetSpl;

        component UtxoNameAssetHasher = Poseidon(2);
        UtxoNameAssetHasher.inputs[0] <== UtxoName.amountSol;
        UtxoNameAssetHasher.inputs[1] <== UtxoName.amountSpl;

        component UtxoNameUtxoCheckHasher = Poseidon(8);
        UtxoNameUtxoCheckHasher.inputs[0] <== 0; // TxVersion
        UtxoNameUtxoCheckHasher.inputs[1] <== UtxoNameAmountHasher.out;
        UtxoNameUtxoCheckHasher.inputs[2] <== UtxoNameAssetHasher.out;
        UtxoNameUtxoCheckHasher.inputs[3] <== UtxoName.blinding;
        UtxoNameUtxoCheckHasher.inputs[4] <== UtxoNameAssetHasher.out;
        UtxoNameUtxoCheckHasher.inputs[5] <== UtxoName.utxoDataHash;
        UtxoNameUtxoCheckHasher.inputs[6] <== 0;
        UtxoNameUtxoCheckHasher.inputs[7] <== UtxoName.pspOwner;
    "#;
        println!("declare_code {}", check_utxo.declare_code);
        // Asserting that the generated declare_code matches the expected output
        assert_eq!(
            remove_formatting(&check_utxo.declare_code),
            remove_formatting(expected_output)
        );

        Ok(())
    }

    #[test]
    fn generate_comparison_check_code_test() -> Result<(), MacroCircomError> {
        let checks = UtxoChecks {
            amount_sol: Some((Comparator::Equal, "sth".to_string())),
            amount_spl: Some((Comparator::Equal, "sth1".to_string())),
            asset_spl: Some((Comparator::Equal, "sth2".to_string())),
            utxo_data_hash: Some((Comparator::Equal, "sth3".to_string())),
            blinding: Some((Comparator::Equal, "sthB".to_string())),
            tx_version: None,
            pool_type: None,
            psp_owner: Some((Comparator::Equal, "sthV".to_string())),
            public_key: Some((Comparator::Equal, "sthPk".to_string())),
        };

        // Setting up a Utxo instance with mock data
        let mut check_utxo = Utxo {
            is_declared: false,
            is_checked: false,
            declare_code: String::new(),
            check_code: String::new(),
            name: "UtxoName".to_string(),
            type_name: "UtxoType".to_string(),
            type_struct: Some(UtxoType {
                fields: vec!["attribute1".to_string(), "attribute2".to_string()],
                name: "UtxoType".to_string(),
                code: String::new(),
            }),
            is_in_utxo: true,
            instruction_name: Some("instruction".to_string()),
            no_utxos: String::from("1"),
            checks: Some(checks),
            utxo_data_checks: Some(vec![(
                "attribute2".to_string(),
                Comparator::Equal,
                "testComparison".to_string(),
            )]),
        };
        check_utxo.generate_check_code()?;
        let expected_output = r#"
component checkUtxoDataAttribute2UtxoName[nIns];
component checkAmountSolUtxoName[nIns];
component checkAmountSplUtxoName[nIns];
component checkAssetSplUtxoName[nIns];
component checkBlindingUtxoName[nIns];
component checkPublicKeyUtxoName[nIns];
component checkAppDataHashUtxoName[nIns];
component checkPspOwnerUtxoName[nIns];

for (var i = 0; i < nIns; i++) {

    checkAmountSolUtxoName[i] = ForceEqualIfEnabled();
    checkAmountSolUtxoName[i].in[0] <== inAmountsHasher[i].inputs[0];
    checkAmountSolUtxoName[i].in[1] <== sth;
    checkAmountSolUtxoName[i].enabled <== isInAppUtxoUtxoName[i] * instruction;

    checkAmountSplUtxoName[i] = ForceEqualIfEnabled();
    checkAmountSplUtxoName[i].in[0] <== inAmountsHasher[i].inputs[1];
    checkAmountSplUtxoName[i].in[1] <== sth1;
    checkAmountSplUtxoName[i].enabled <== isInAppUtxoUtxoName[i] * instruction;

    checkAssetSplUtxoName[i] = ForceEqualIfEnabled();
    checkAssetSplUtxoName[i].in[0] <== inAssetsHasher[i].inputs[1];
    checkAssetSplUtxoName[i].in[1] <== sth2;
    checkAssetSplUtxoName[i].enabled <== isInAppUtxoUtxoName[i] * instruction;

    checkBlindingUtxoName[i] = ForceEqualIfEnabled();
    checkBlindingUtxoName[i].in[0] <== inCommitmentHasher[i].inputs[3];
    checkBlindingUtxoName[i].in[1] <== sthB;
    checkBlindingUtxoName[i].enabled <== isInAppUtxoUtxoName[i] * instruction;

    checkPublicKeyUtxoName[i] = ForceEqualIfEnabled();
    checkPublicKeyUtxoName[i].in[0] <== inCommitmentHasher[i].inputs[2];
    checkPublicKeyUtxoName[i].in[1] <== sthPk;
    checkPublicKeyUtxoName[i].enabled <== isInAppUtxoUtxoName[i] * instruction;

    checkAppDataHashUtxoName[i] = ForceEqualIfEnabled();
    checkAppDataHashUtxoName[i].in[0] <== inAppDataHash[i];
    checkAppDataHashUtxoName[i].in[1] <== sth3;
    checkAppDataHashUtxoName[i].enabled <== isInAppUtxoUtxoName[i] * instruction;

    checkPspOwnerUtxoName[i] = ForceEqualIfEnabled();
    checkPspOwnerUtxoName[i].in[0] <== inCommitmentHasher[i].inputs[7];
    checkPspOwnerUtxoName[i].in[1] <== sthV;
    checkPspOwnerUtxoName[i].enabled <== isInAppUtxoUtxoName[i] * instruction;

    checkUtxoDataAttribute2UtxoName[i] = ForceEqualIfEnabled();
    checkUtxoDataAttribute2UtxoName[i].in[0] <== UtxoName.attribute2;
    checkUtxoDataAttribute2UtxoName[i].in[1] <== testComparison;
    checkUtxoDataAttribute2UtxoName[i].enabled <== isInAppUtxoUtxoName[i] * instruction;

}
"#;
        println!("check_utxo.check_code {}", check_utxo.check_code);
        // Asserting that the generated check_code matches the expected output
        assert_eq!(
            remove_formatting(&check_utxo.check_code),
            remove_formatting(expected_output)
        );

        Ok(())
    }

    #[test]
    fn complete_test() {
        let utxo_type = UtxoType {
            fields: vec!["attribute1".to_string(), "attribute2".to_string()],
            name: "UtxoType".to_string(),
            code: String::new(),
        };
        let contents = String::from(
            "inUtxo utxoName
            {
                type: UtxoType,
                enabled: instruction1,
                checks: {
                    amountSol == sth,
                    amountSpl == sth1,
                    assetSpl == sth2,
                    utxoDataHash == sth3,
                    blinding == sth,
                },
                dataChecks: {
                    attribute2 == testComparison,
                },
           }",
        );
        let parsing_res = match crate::lang::ParseInstanceParser::new().parse(&contents) {
            Ok(instance) => instance,
            Err(error) => {
                println!("Parsing check utxo error.");
                panic!("{}", describe_error(&contents, error));
            }
        };

        let mut checked_utxos = parsing_res.2;
        assign_utxo_type(&vec![utxo_type], &mut checked_utxos).unwrap();

        generate_check_utxo_code(&mut checked_utxos).unwrap();
        let check_utxo = checked_utxos[0].clone();
        assert_eq!(check_utxo.name, "utxoName");
        assert_eq!(check_utxo.no_utxos, "1");
        assert_eq!(
            check_utxo.instruction_name,
            Some(String::from("instruction1"))
        );
        assert_eq!(
            check_utxo.checks.as_ref().unwrap().amount_sol,
            Some((Comparator::Equal, String::from("sth")))
        );
        assert_eq!(
            check_utxo.checks.as_ref().unwrap().amount_spl,
            Some((Comparator::Equal, String::from("sth1")))
        );
        assert_eq!(
            check_utxo.checks.as_ref().unwrap().asset_spl,
            Some((Comparator::Equal, String::from("sth2")))
        );
        assert_eq!(
            check_utxo.checks.as_ref().unwrap().utxo_data_hash,
            Some((Comparator::Equal, String::from("sth3")))
        );
        assert_eq!(
            check_utxo.utxo_data_checks,
            Some(vec![(
                "attribute2".to_string(),
                Comparator::Equal,
                "testComparison".to_string(),
            ),])
        );
    }

    #[test]
    fn complete_test_2() {
        let utxo_type = UtxoType {
            fields: vec!["attribute1".to_string(), "attribute2".to_string()],
            name: "UtxoType".to_string(),
            code: String::new(),
        };
        let contents = String::from(
            "pragma circom 2.1.4;
            include \"../../node_modules/circomlib/circuits/poseidon.circom\";

           outUtxo utxoName
       {
            type: UtxoType,
            checks: {
                amountSol == sthSol,
                amountSpl == sthSpl,
                assetSpl == sthAsset,
                utxoDataHash == sthApp,
                pspOwner== sthV,
                blinding == sthB,
            },

            dataChecks: {
                attribute21 == testComparison1,
               },
           }
           inUtxo utxoName1
            {
                type: UtxoType,
                checks: {
                    amountSol == sth,
                    amountSpl == sth1,
                    assetSpl == sth2,
                    utxoDataHash == sth3,
                    blinding == sth,
                },
                dataChecks: {
                    attribute2 == testComparison,
                },
           }
   ",
        );
        let parsing_res = match crate::lang::ParseInstanceParser::new().parse(&contents) {
            Ok(instance) => instance,
            Err(error) => {
                println!("Parsing check utxo error.");
                panic!("{}", describe_error(&contents, error));
            }
        };

        let mut checked_utxos = parsing_res.2;
        assign_utxo_type(&vec![utxo_type], &mut checked_utxos).unwrap();
        generate_check_utxo_code(&mut checked_utxos).unwrap();
        let check_utxo = checked_utxos[0].clone();
        assert_eq!(checked_utxos.len(), 2);
        assert_eq!(check_utxo.name, "utxoName");
        assert_eq!(check_utxo.no_utxos, "1");
        assert_eq!(check_utxo.instruction_name, None);
        assert_eq!(
            check_utxo.checks.as_ref().unwrap().amount_sol,
            Some((Comparator::Equal, String::from("sthSol")))
        );
        assert_eq!(
            check_utxo.checks.as_ref().unwrap().amount_spl,
            Some((Comparator::Equal, String::from("sthSpl")))
        );
        assert_eq!(
            check_utxo.checks.as_ref().unwrap().asset_spl,
            Some((Comparator::Equal, String::from("sthAsset")))
        );
        assert_eq!(
            check_utxo.checks.as_ref().unwrap().utxo_data_hash,
            Some((Comparator::Equal, String::from("sthApp")))
        );
        assert_eq!(
            check_utxo.checks.as_ref().unwrap().blinding,
            Some((Comparator::Equal, String::from("sthB")))
        );
        assert_eq!(
            check_utxo.checks.as_ref().unwrap().psp_owner,
            Some((Comparator::Equal, String::from("sthV")))
        );
        assert_eq!(
            check_utxo.utxo_data_checks,
            Some(vec![(
                "attribute21".to_string(),
                Comparator::Equal,
                "testComparison1".to_string(),
            ),])
        );

        let check_utxo1 = checked_utxos[1].clone();
        assert_eq!(check_utxo1.name, "utxoName1");
        assert_eq!(check_utxo1.no_utxos, "1");
        assert_eq!(check_utxo1.instruction_name, None);
        assert_eq!(
            check_utxo1.checks.as_ref().unwrap().amount_sol,
            Some((Comparator::Equal, String::from("sth")))
        );
        assert_eq!(
            check_utxo1.checks.as_ref().unwrap().amount_spl,
            Some((Comparator::Equal, String::from("sth1")))
        );
        assert_eq!(
            check_utxo1.checks.as_ref().unwrap().asset_spl,
            Some((Comparator::Equal, String::from("sth2")))
        );
        assert_eq!(
            check_utxo1.checks.as_ref().unwrap().utxo_data_hash,
            Some((Comparator::Equal, String::from("sth3")))
        );
        assert_eq!(
            check_utxo1.utxo_data_checks,
            Some(vec![(
                "attribute2".to_string(),
                Comparator::Equal,
                "testComparison".to_string(),
            ),])
        );
        println!("check_utxo1 check_code {}", check_utxo1.check_code);
    }
}
