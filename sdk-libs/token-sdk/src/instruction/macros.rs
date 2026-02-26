macro_rules! impl_with_top_up {
    ($base:ident, $with_top_up:ident) => {
        impl $base {
            pub fn with_max_top_up(self, max_top_up: u16) -> $with_top_up {
                $with_top_up {
                    inner: self,
                    max_top_up,
                }
            }

            pub fn instruction(self) -> Result<Instruction, ProgramError> {
                self.build_instruction(None)
            }
        }

        pub struct $with_top_up {
            inner: $base,
            max_top_up: u16,
        }

        impl $with_top_up {
            pub fn instruction(self) -> Result<Instruction, ProgramError> {
                self.inner.build_instruction(Some(self.max_top_up))
            }
        }
    };
}
