#[test]
    fn test_syscall_alt_bn128_group_ops() {
        use solana_sdk::alt_bn128::prelude::{ADD, MUL, PAIRING};

        let config = Config::default();
        prepare_mockup!(
            invoke_context,
            transaction_context,
            program_id,
            bpf_loader::id(),
        );

        let left_point: [u8; 64] = [
            45, 206, 255, 166, 152, 55, 128, 138, 79, 217, 145, 164, 25, 74, 120, 234, 234, 217,
            68, 149, 162, 44, 133, 120, 184, 205, 12, 44, 175, 98, 168, 172, 20, 24, 216, 15, 209,
            175, 106, 75, 147, 236, 90, 101, 123, 219, 245, 151, 209, 202, 218, 104, 148, 8, 32,
            254, 243, 191, 218, 122, 42, 81, 193, 84,
        ];
        let right_point: [u8; 64] = [
            41, 139, 183, 208, 246, 198, 118, 127, 89, 160, 9, 27, 61, 26, 123, 180, 221, 108, 17,
            166, 47, 115, 82, 48, 132, 139, 253, 65, 152, 92, 209, 53, 37, 25, 83, 61, 252, 42,
            181, 243, 16, 21, 2, 199, 123, 96, 218, 151, 253, 86, 69, 181, 202, 109, 64, 129, 124,
            254, 192, 25, 177, 199, 26, 50,
        ];
        let add_input: [u8; 128] = [left_point, right_point].concat().try_into().unwrap();
        let add_input_va = 0x100000000;

        let invalid_add_input: [u8; 128] = [left_point, left_point].concat().try_into().unwrap();
        let invalid_add_input_va = 0x200000000;

        let scalar: [u8; 32] = [
            34, 238, 251, 182, 234, 248, 214, 189, 46, 67, 42, 25, 71, 58, 145, 58, 61, 28, 116,
            110, 60, 17, 82, 149, 178, 187, 160, 211, 37, 226, 174, 231,
        ];
        let mul_input: [u8; 96] = [left_point.to_vec(), scalar.to_vec()]
            .concat()
            .try_into()
            .unwrap();
        let mul_input_va = 0x300000000;

        let scalar: [u8; 32] = [0u8; 32];
        let invalid_mul_input: [u8; 96] = [right_point.to_vec(), scalar.to_vec()]
            .concat()
            .try_into()
            .unwrap();
        let invalid_mul_input_va = 0x400000000;

        // pairing input for 4 pairings
        let pairing_input: [u8; 768] = [
            45, 206, 255, 166, 152, 55, 128, 138, 79, 217, 145, 164, 25, 74, 120, 234, 234, 217,
            68, 149, 162, 44, 133, 120, 184, 205, 12, 44, 175, 98, 168, 172, 28, 75, 118, 99, 15,
            130, 53, 222, 36, 99, 235, 81, 5, 165, 98, 197, 197, 182, 144, 40, 212, 105, 169, 142,
            72, 96, 177, 156, 174, 43, 59, 243, 40, 57, 233, 205, 180, 46, 35, 111, 215, 5, 23, 93,
            12, 71, 118, 225, 7, 46, 247, 147, 47, 130, 106, 189, 184, 80, 146, 103, 141, 52, 242,
            25, 0, 203, 124, 176, 110, 34, 151, 212, 66, 180, 238, 151, 236, 189, 133, 209, 17,
            137, 205, 183, 168, 196, 92, 159, 75, 174, 81, 168, 18, 86, 176, 56, 16, 26, 210, 20,
            18, 81, 122, 142, 104, 62, 251, 169, 98, 141, 21, 253, 50, 130, 182, 15, 33, 109, 228,
            31, 79, 183, 88, 147, 174, 108, 4, 22, 14, 129, 168, 6, 80, 246, 254, 100, 218, 131,
            94, 49, 247, 211, 3, 245, 22, 200, 177, 91, 60, 144, 147, 174, 90, 17, 19, 189, 62,
            147, 152, 18, 36, 20, 77, 212, 52, 161, 196, 229, 247, 147, 81, 142, 46, 67, 114, 86,
            53, 61, 83, 139, 62, 9, 125, 2, 200, 160, 22, 242, 170, 105, 142, 199, 10, 168, 124,
            147, 159, 41, 16, 215, 178, 145, 109, 204, 123, 106, 227, 188, 113, 23, 102, 39, 82,
            136, 108, 252, 201, 126, 84, 122, 103, 165, 107, 89, 25, 142, 147, 147, 146, 13, 72,
            58, 114, 96, 191, 183, 49, 251, 93, 37, 241, 170, 73, 51, 53, 169, 231, 18, 151, 228,
            133, 183, 174, 243, 18, 194, 24, 0, 222, 239, 18, 31, 30, 118, 66, 106, 0, 102, 94, 92,
            68, 121, 103, 67, 34, 212, 247, 94, 218, 221, 70, 222, 189, 92, 217, 146, 246, 237, 9,
            6, 137, 208, 88, 95, 240, 117, 236, 158, 153, 173, 105, 12, 51, 149, 188, 75, 49, 51,
            112, 179, 142, 243, 85, 172, 218, 220, 209, 34, 151, 91, 18, 200, 94, 165, 219, 140,
            109, 235, 74, 171, 113, 128, 141, 203, 64, 143, 227, 209, 231, 105, 12, 67, 211, 123,
            76, 230, 204, 1, 102, 250, 125, 170, 41, 139, 183, 208, 246, 198, 118, 127, 89, 160, 9,
            27, 61, 26, 123, 180, 221, 108, 17, 166, 47, 115, 82, 48, 132, 139, 253, 65, 152, 92,
            209, 53, 37, 25, 83, 61, 252, 42, 181, 243, 16, 21, 2, 199, 123, 96, 218, 151, 253, 86,
            69, 181, 202, 109, 64, 129, 124, 254, 192, 25, 177, 199, 26, 50, 25, 142, 147, 147,
            146, 13, 72, 58, 114, 96, 191, 183, 49, 251, 93, 37, 241, 170, 73, 51, 53, 169, 231,
            18, 151, 228, 133, 183, 174, 243, 18, 194, 24, 0, 222, 239, 18, 31, 30, 118, 66, 106,
            0, 102, 94, 92, 68, 121, 103, 67, 34, 212, 247, 94, 218, 221, 70, 222, 189, 92, 217,
            146, 246, 237, 9, 6, 137, 208, 88, 95, 240, 117, 236, 158, 153, 173, 105, 12, 51, 149,
            188, 75, 49, 51, 112, 179, 142, 243, 85, 172, 218, 220, 209, 34, 151, 91, 18, 200, 94,
            165, 219, 140, 109, 235, 74, 171, 113, 128, 141, 203, 64, 143, 227, 209, 231, 105, 12,
            67, 211, 123, 76, 230, 204, 1, 102, 250, 125, 170, 45, 77, 154, 167, 227, 2, 217, 223,
            65, 116, 157, 85, 7, 148, 157, 5, 219, 234, 51, 251, 177, 108, 100, 59, 34, 245, 153,
            162, 190, 109, 242, 226, 20, 190, 221, 80, 60, 55, 206, 176, 97, 216, 236, 96, 32, 159,
            227, 69, 206, 137, 131, 10, 25, 35, 3, 1, 240, 118, 202, 255, 0, 77, 25, 38, 9, 103, 3,
            47, 203, 247, 118, 209, 175, 201, 133, 248, 136, 119, 241, 130, 211, 132, 128, 166, 83,
            242, 222, 202, 169, 121, 76, 188, 59, 243, 6, 12, 14, 24, 120, 71, 173, 76, 121, 131,
            116, 208, 214, 115, 43, 245, 1, 132, 125, 214, 139, 192, 224, 113, 36, 30, 2, 19, 188,
            127, 193, 61, 183, 171, 48, 76, 251, 209, 224, 138, 112, 74, 153, 245, 232, 71, 217,
            63, 140, 60, 170, 253, 222, 196, 107, 122, 13, 55, 157, 166, 154, 77, 17, 35, 70, 167,
            23, 57, 193, 177, 164, 87, 168, 199, 49, 49, 35, 210, 77, 47, 145, 146, 248, 150, 183,
            198, 62, 234, 5, 169, 213, 127, 6, 84, 122, 208, 206, 200,
        ];
        let pairing_input_va = 0x500000000;

        let result_point: [u8; 64] = [0u8; 64];
        let result_point_va = 0x600000000;

        let mut memory_mapping = MemoryMapping::new(
            vec![
                MemoryRegion {
                    host_addr: add_input.as_ptr() as *const _ as u64,
                    vm_addr: add_input_va,
                    len: 128,
                    vm_gap_shift: 63,
                    is_writable: false,
                },
                MemoryRegion {
                    host_addr: invalid_add_input.as_ptr() as *const _ as u64,
                    vm_addr: invalid_add_input_va,
                    len: 128,
                    vm_gap_shift: 63,
                    is_writable: false,
                },
                MemoryRegion {
                    host_addr: mul_input.as_ptr() as *const _ as u64,
                    vm_addr: mul_input_va,
                    len: 96,
                    vm_gap_shift: 63,
                    is_writable: false,
                },
                MemoryRegion {
                    host_addr: invalid_mul_input.as_ptr() as *const _ as u64,
                    vm_addr: invalid_mul_input_va,
                    len: 96,
                    vm_gap_shift: 63,
                    is_writable: false,
                },
                MemoryRegion {
                    host_addr: pairing_input.as_ptr() as *const _ as u64,
                    vm_addr: pairing_input_va,
                    len: 768,
                    vm_gap_shift: 63,
                    is_writable: false,
                },
                MemoryRegion {
                    host_addr: result_point.as_ptr() as *const _ as u64,
                    vm_addr: result_point_va,
                    len: 64,
                    vm_gap_shift: 63,
                    is_writable: true,
                },
            ],
            &config,
        )
        .unwrap();

        invoke_context
            .get_compute_meter()
            .borrow_mut()
            .mock_set_remaining(
                (invoke_context.get_compute_budget().alt_bn128_addition_cost
                    + invoke_context
                        .get_compute_budget()
                        .alt_bn128_multiplication_cost)
                    * 2
                    + invoke_context
                        .get_compute_budget()
                        .alt_bn128_pairing_one_pair_cost_first
                    + invoke_context
                        .get_compute_budget()
                        .alt_bn128_pairing_one_pair_cost_other
                        * 3
                    + invoke_context.get_compute_budget().sha256_base_cost
                    + pairing_input.len() as u64
                    + ALT_BN128_PAIRING_OUTPUT_LEN as u64,
            );

        let mut result = ProgramResult::Ok(0);
        SyscallAltBn128::call(
            &mut invoke_context,
            ADD,
            add_input_va,
            128u64,
            result_point_va,
            0u64,
            &mut memory_mapping,
            &mut result,
        );

        assert_eq!(0, result.unwrap());
        let expected_sum = [
            6, 207, 172, 91, 17, 102, 155, 29, 172, 120, 116, 58, 50, 31, 77, 59, 69, 210, 171, 94,
            62, 145, 17, 192, 81, 168, 227, 111, 130, 203, 179, 5, 24, 124, 198, 161, 73, 7, 32,
            119, 46, 203, 168, 234, 246, 107, 2, 58, 161, 254, 37, 50, 142, 78, 96, 177, 87, 36,
            185, 2, 35, 109, 204, 254,
        ];

        assert_eq!(expected_sum, result_point);

        let mut result = ProgramResult::Ok(0);
        SyscallAltBn128::call(
            &mut invoke_context,
            ADD,
            invalid_add_input_va,
            128u64,
            result_point_va,
            0u64,
            &mut memory_mapping,
            &mut result,
        );
        assert_eq!(0, result.unwrap());
        assert!(expected_sum != result_point);

        let expected_product = [
            22, 62, 73, 246, 147, 86, 205, 146, 228, 167, 174, 97, 242, 13, 212, 192, 52, 160, 85,
            35, 174, 196, 120, 217, 139, 25, 205, 126, 148, 29, 105, 141, 11, 84, 142, 38, 52, 205,
            0, 231, 253, 214, 133, 137, 66, 236, 51, 236, 199, 234, 64, 38, 250, 151, 99, 76, 217,
            84, 73, 165, 70, 197, 122, 69,
        ];

        let mut result = ProgramResult::Ok(0);
        SyscallAltBn128::call(
            &mut invoke_context,
            MUL,
            mul_input_va,
            96u64,
            result_point_va,
            0u64,
            &mut memory_mapping,
            &mut result,
        );
        assert_eq!(0, result.unwrap());
        assert_eq!(expected_product, result_point);

        let mut result = ProgramResult::Ok(0);
        SyscallAltBn128::call(
            &mut invoke_context,
            MUL,
            invalid_mul_input_va,
            96u64,
            result_point_va,
            0u64,
            &mut memory_mapping,
            &mut result,
        );
        assert_eq!(0, result.unwrap());
        assert!(expected_product != result_point);

        let mut result = ProgramResult::Ok(0);
        SyscallAltBn128::call(
            &mut invoke_context,
            PAIRING,
            pairing_input_va,
            768u64,
            result_point_va,
            0u64,
            &mut memory_mapping,
            &mut result,
        );
        assert_eq!(0, result.unwrap());
        assert_eq!(1u8, result_point[31]);

        let mut result = ProgramResult::Ok(0);
        SyscallAltBn128::call(
            &mut invoke_context,
            MUL,
            mul_input_va,
            96u64,
            result_point_va,
            0u64,
            &mut memory_mapping,
            &mut result,
        );
        assert!(matches!(
            result,
            ProgramResult::Err(EbpfError::UserError(error)) if error.downcast_ref::<BpfError>().unwrap() == &BpfError::SyscallError(
                SyscallError::InstructionError(InstructionError::ComputationalBudgetExceeded)
            ),
        ));
    }
