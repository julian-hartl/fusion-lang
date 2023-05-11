#[macro_export]
macro_rules! id {
    ($name:ident) => {
        #[derive(Debug, Clone, Eq, PartialEq, Hash, Copy)]
        pub struct $name {
            pub index: u64,
        }

        impl $name {
            pub fn new(index: u64) -> Self {
                $name {
                    index,
                }
            }
        }
    };
}

#[macro_export]
macro_rules! id_generator {
    ($name:ident, $id:ident) => {
        #[derive(Debug, Clone, Eq, PartialEq, Hash)]
        pub struct $name {
            next_index: u64,
        }

        impl $name {
            pub fn new() -> Self {
                $name {
                    next_index: 0,
                }
            }

            pub fn next(&mut self) -> $id {
                let index = self.next_index;
                self.next_index += 1;
                $id::new(index)
            }
        }
    };
}

pub type Result<T> = std::result::Result<T, ()>;