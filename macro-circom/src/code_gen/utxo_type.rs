use std::collections::HashSet;

use heck::ToUpperCamelCase;

use crate::errors::MacroCircomError;
// TODO: add check that a Utxo type exists for every CheckUtxo
#[derive(PartialEq, Debug, Clone)]
pub struct UtxoType {
    pub name: String,
    pub fields: Vec<String>,
    pub code: String,
}

pub fn get_native_utxo_type() -> UtxoType {
    UtxoType {
        name: "native".to_string(),
        fields: Vec::new(),
        code: String::new(),
    }
}

impl UtxoType {
    fn generate_code(&mut self) -> Result<(), MacroCircomError> {
        let template = r#"
template {{name}}() {
    {{#each fields as |field|}}
    signal input {{field}}In;
    signal output {{field}};
    {{field}} <== {{field}}In;
    {{/each}}
}"#;

        let handlebars = handlebars::Handlebars::new();
        let mut utxo_fields = vec![
            "owner",
            "blinding",
            "address",
            "metaHash",
            "dataHash",
            "amountSol",
            "amountSpl",
            "assetSpl",
            "version",
            "type",
        ];
        // adding custom fields to native utxo fields
        for field in &self.fields {
            utxo_fields.push(field)
        }

        let data = serde_json::json!({
            "name": self.name.to_upper_camel_case(),
            "fields": &utxo_fields,
        });

        match handlebars.render_template(template, &data) {
            Ok(res) => {
                self.code = res;
                Ok(())
            }
            Err(err) => Err(MacroCircomError::CodeGenerationFailed(
                format!("UtxoType {}", self.name),
                format!("{}", err),
            )),
        }
    }
}

pub fn generate_utxo_type_code(utxo_types: &mut Vec<UtxoType>) -> Result<(), MacroCircomError> {
    check_for_duplicates(utxo_types)?;
    for utxo in utxo_types {
        utxo.generate_code()?;
    }

    Ok(())
}

fn check_for_duplicates(v: &Vec<UtxoType>) -> Result<(), MacroCircomError> {
    let mut seen = HashSet::new();

    for item in v {
        if !seen.insert(&item.name) {
            return Err(MacroCircomError::DuplicateUtxoType(item.name.clone()));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::remove_formatting;

    #[test]
    fn test_utxo_type_code_generation() {
        let mut my_utxo = UtxoType {
            name: "MyUtxo".to_string(),
            fields: vec!["field1".to_string(), "field2".to_string()],
            code: String::new(),
        };

        my_utxo.generate_code().unwrap();

        let expected_code = r#"
template MyUtxo() {
    signal input ownerIn;
	signal output owner;
	owner <== ownerIn;
	signal input blindingIn;
	signal output blinding;
	blinding <== blindingIn;
	signal input addressIn;
	signal output address;
	address <== addressIn;
    signal input metaHashIn;
	signal output metaHash;
	metaHash <== metaHashIn;
	signal input dataHashIn;
	signal output dataHash;
	dataHash <== dataHashIn;
	signal input amountSolIn;
	signal output amountSol;
	amountSol <== amountSolIn;
	signal input amountSplIn;
	signal output amountSpl;
	amountSpl <== amountSplIn;
	signal input assetSplIn;
	signal output assetSpl;
	assetSpl <== assetSplIn;
	signal input versionIn;
	signal output version;
	version <== versionIn;
	signal input typeIn;
	signal output type;
	type <== typeIn;
    signal input field1In;
    signal output field1;
    field1 <== field1In;
    
    signal input field2In;
    signal output field2;
    field2 <== field2In;
}
"#;

        assert_eq!(
            remove_formatting(my_utxo.code.as_str()),
            remove_formatting(expected_code)
        );
    }
}
