use anchor_lang::prelude::AccountInfo;

pub trait InvokeCpiAccounts<'info> {
    fn get_invoking_program(&self) -> &AccountInfo<'info>;
}


// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn it_works() {
//         let result = add(2, 2);
//         assert_eq!(result, 4);
//     }
// }
