  
  
Derive Macros:
1. LightAccount
2. LightAccounts
3. light_program

  
  
  enum AccountType {
      Pda,
      PdaZeroCopy,
      Token,
      Ata,
      Mint,
  }

  trait LightAccountVariant {
      const TYPE: AccountType;
      type Seeds;
      type Data;
  }

  Generated Structs

  // PDA
  struct UserRecordVariant {
      seeds: UserRecordSeeds,
      data: UserRecord,
  }

  impl LightAccountVariant for UserRecordVariant {
      const TYPE: AccountType = AccountType::Pda;
      type Seeds = UserRecordSeeds;
      type Data = UserRecord;
  }

  // Token
  struct VaultVariant {
      seeds: VaultSeeds,
      data: TokenData,
  }

  impl LightAccountVariant for VaultVariant {
      const TYPE: AccountType = AccountType::Token;
      type Seeds = VaultSeeds;
      type Data = TokenData;
  }

  Packed Versions

  trait PackedLightAccount {
      const TYPE: AccountType;
      type Seeds;
      type Data;
      type Unpacked: LightAccountVariant;
  }

  struct PackedUserRecordVariant {
      seeds: PackedUserRecordSeeds,
      data: PackedUserRecord,
  }

  impl PackedLightAccount for PackedUserRecordVariant {
      const TYPE: AccountType = AccountType::Pda;
      type Seeds = PackedUserRecordSeeds;
      type Data = PackedUserRecord;
      type Unpacked = UserRecordVariant;
  }

  Enum

  enum LightAccountVariant {
      UserRecord(UserRecordVariant),
      Vault(VaultVariant),
  }

  impl LightAccountVariant {
      fn account_type(&self) -> AccountType {
          match self {
              Self::UserRecord(_) => UserRecordVariant::TYPE,
              Self::Vault(_) => VaultVariant::TYPE,
          }
      }
  }
