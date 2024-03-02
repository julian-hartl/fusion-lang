use rustc_hash::FxHashMap;

use crate::codegen::machine::{Abi, PhysicalRegister, VirtualRegister};
use crate::codegen::register_allocator::RegisterAllocator;
use firc_middle::ty::Type;

pub struct GreedyRegisterAllocator<A: Abi> {
    map: FxHashMap<VirtualRegister, A::REG>,
    available: Vec<A::REG>,
}

impl<A: Abi> RegisterAllocator<A> for GreedyRegisterAllocator<A> where <A as Abi>::REG: 'static{
    fn new() -> Self {
        Self {
            map: FxHashMap::default(),
            available: A::REG::all().to_vec().into_iter().filter(
                |reg| reg.is_gp()
            ).collect(),
        }
    }

    fn allocate(&mut self, vreg: VirtualRegister, ty: Type) -> A::REG {
        let mut reg = None;
        for r in &self.available {
            // todo: account for arbitrary sized integers
            if r.size() == ty.size() {
                reg = Some(r);
                break;
            }
        }
        let reg = reg.copied().expect("No available registers");
        self.map.insert(vreg, reg);
        self.live(reg);
        reg
    }

    fn get(&self, vreg: VirtualRegister) -> A::REG {
        *self.map.get(&vreg).expect("No register for virtual register")
    }

    fn live(&mut self, reg: A::REG) {
        self.available.retain(|r| *r != reg);
    }

    fn kill(&mut self, reg: A::REG) {
        self.available.push(reg);
    }
}