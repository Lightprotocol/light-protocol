#[cfg(test)]
mod tests {
    use serde_derive::{Deserialize, Serialize};
    use serde_json;

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct TestStruct {
        #[serde(
            deserialize_with = "photon_api::string_u64::direct::deserialize",
            serialize_with = "photon_api::string_u64::direct::serialize"
        )]
        amount: u64,
        #[serde(
            deserialize_with = "photon_api::string_u64::u32_direct::deserialize",
            serialize_with = "photon_api::string_u64::u32_direct::serialize"
        )]
        index: u32,
        #[serde(
            deserialize_with = "photon_api::string_u64::u16_direct::deserialize",
            serialize_with = "photon_api::string_u64::u16_direct::serialize"
        )]
        root_index: u16,
    }

    #[test]
    fn test_deserialize_from_string() {
        // Test deserializing from string values (new Photon API format)
        let json_str = r#"{"amount":"5106734359795461623","index":"42","root_index":"7"}"#;
        let result: TestStruct = serde_json::from_str(json_str).unwrap();

        assert_eq!(result.amount, 5106734359795461623);
        assert_eq!(result.index, 42);
        assert_eq!(result.root_index, 7);
    }

    #[test]
    fn test_deserialize_from_number() {
        // Test deserializing from numeric values (backward compatibility)
        let json_str = r#"{"amount":5106734359795461623,"index":42,"root_index":7}"#;
        let result: TestStruct = serde_json::from_str(json_str).unwrap();

        assert_eq!(result.amount, 5106734359795461623);
        assert_eq!(result.index, 42);
        assert_eq!(result.root_index, 7);
    }

    #[test]
    fn test_serialize_to_string() {
        // Test that serialization produces strings
        let test_struct = TestStruct {
            amount: 5106734359795461623,
            index: 42,
            root_index: 7,
        };

        let json = serde_json::to_string(&test_struct).unwrap();
        assert!(json.contains(r#""amount":"5106734359795461623""#));
        assert!(json.contains(r#""index":"42""#));
        assert!(json.contains(r#""root_index":"7""#));
    }

    #[test]
    fn test_roundtrip() {
        let original = TestStruct {
            amount: u64::MAX,
            index: u32::MAX,
            root_index: u16::MAX,
        };

        let json = serde_json::to_string(&original).unwrap();
        let deserialized: TestStruct = serde_json::from_str(&json).unwrap();

        assert_eq!(original, deserialized);
    }

    #[test]
    fn test_optional_fields() {
        #[derive(Debug, Serialize, Deserialize, PartialEq)]
        struct OptionalStruct {
            #[serde(
                deserialize_with = "photon_api::string_u64::option_direct::deserialize",
                serialize_with = "photon_api::string_u64::option_direct::serialize",
                default
            )]
            seq: Option<u64>,
        }

        // Test with Some value as string
        let json_str = r#"{"seq":"123456789"}"#;
        let result: OptionalStruct = serde_json::from_str(json_str).unwrap();
        assert_eq!(result.seq, Some(123456789));

        // Test with None
        let json_str = r#"{}"#;
        let result: OptionalStruct = serde_json::from_str(json_str).unwrap();
        assert_eq!(result.seq, None);

        // Test with null
        let json_str = r#"{"seq":null}"#;
        let result: OptionalStruct = serde_json::from_str(json_str).unwrap();
        assert_eq!(result.seq, None);
    }
}
