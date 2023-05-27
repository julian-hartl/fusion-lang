#[macro_export]
macro_rules! idx {
    ($name:ident) => {
        #[derive(Debug, Clone, Eq, PartialEq, Hash, Copy)]
        pub struct $name {
            index: usize,
        }

        impl Idx for $name {
            fn as_idx(&self) -> usize {
                self.index
            }

            fn new(index: usize) -> Self {
                Self {
                    index,
                }
            }
        }
    };
}

pub type Result<T> = std::result::Result<T, ()>;

pub trait Idx {
    fn as_idx(&self) -> usize;
    fn new(index: usize) -> Self;
}

pub struct IdxVec<Index, T> where Index: Idx {
    vec: Vec<T>,
    _marker: std::marker::PhantomData<Index>,
}

impl <Index, T> IdxVec<Index, T> where Index: Idx {
    pub fn new() -> Self {
        Self {
            vec: vec![],
            _marker: std::marker::PhantomData,
        }
    }

    pub fn push(&mut self, value: T) -> Index {
        let index = Index::new(self.vec.len());
        self.vec.push(value);
        index
    }

    pub fn get(&self, index: Index) -> &T {
        self.vec.get(index.as_idx()).unwrap()
    }

    pub fn get_mut(&mut self, index: Index) -> &mut T {
        self.vec.get_mut(index.as_idx()).unwrap()
    }

    pub fn iter(&self) -> impl Iterator<Item=&T> {
        self.vec.iter()
    }

    pub fn indexed_iter(&self) -> impl Iterator<Item=(Index, &T)> {
        self.vec.iter().enumerate().map(|(i, v)| (Index::new(i), v))
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item=&mut T> {
        self.vec.iter_mut()
    }

    pub fn as_vec(&self) -> &Vec<T> {
        &self.vec
    }

    pub fn last(&self) -> Option<&T> {
        self.vec.last()
    }

    pub fn last_mut(&mut self) -> Option<&mut T> {
        self.vec.last_mut()
    }

    pub fn first(&self) -> Option<&T> {
        self.vec.first()
    }
}

impl <Index, T> std::ops::Index<Index> for IdxVec<Index, T> where Index: Idx {
    type Output = T;

    fn index(&self, index: Index) -> &Self::Output {
        self.get(index)
    }
}

impl <Index, T> std::ops::IndexMut<Index> for IdxVec<Index, T> where Index: Idx {
    fn index_mut(&mut self, index: Index) -> &mut Self::Output {
        self.get_mut(index)
    }
}

impl <Index, T> std::ops::Deref for IdxVec<Index, T> where Index: Idx {
    type Target = Vec<T>;

    fn deref(&self) -> &Self::Target {
        &self.vec
    }
}

impl <Index, T> std::ops::DerefMut for IdxVec<Index, T> where Index: Idx {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.vec
    }
}

impl <Index, T> Clone for IdxVec<Index, T> where Index: Idx, T: Clone {
    fn clone(&self) -> Self {
        Self {
            vec: self.vec.clone(),
            _marker: std::marker::PhantomData,
        }
    }
}