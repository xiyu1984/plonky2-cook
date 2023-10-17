#![allow(incomplete_features)]

// use log::{info, Level};
use core::marker::PhantomData;

use plonky2::field::extension::Extendable;
use plonky2::field::packed::PackedField;
use plonky2::gates::gate::Gate;
use plonky2::gates::packed_util::PackedEvaluableBase;
use plonky2::gates::util::StridedConstraintConsumer;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;
use plonky2::iop::generator::{GeneratedValues, SimpleGenerator, WitnessGeneratorRef};
use plonky2::iop::target::Target;
use plonky2::iop::witness::{PartitionWitness, Witness, WitnessWrite};
use plonky2::plonk::circuit_builder::CircuitBuilder;
use plonky2::plonk::circuit_data::{CircuitConfig, CommonCircuitData};
use plonky2::plonk::vars::{
    EvaluationTargets, EvaluationVars, EvaluationVarsBase, EvaluationVarsBaseBatch,
    EvaluationVarsBasePacked,
};
use plonky2::util::serialization::{Buffer, IoResult, Read, Write};

// A test gate whick can perform `x1 * x2 + x3 + x4 + x5`
#[derive(Debug, Clone)]
pub struct SimpleMulAddTestGate {
    pub num_ops: usize,
}

impl SimpleMulAddTestGate {
    pub fn new_from_config(config: &CircuitConfig) -> Self {
        Self {
            num_ops: Self::num_ops(config),
        }
    }

    /// Determine the maximum number of operations that can fit in one gate for the given config.
    pub(crate) fn num_ops(config: &CircuitConfig) -> usize {
        config.num_routed_wires / Self::wires_per_op()
    }

    // We can treat wires as variables and outputs
    pub fn wires_per_op() -> usize {
        6
    }

    // get the index of 
    pub fn wire_ith_multiplicand_0(i: usize) -> usize {
        Self::wires_per_op() * i
    }
    pub fn wire_ith_multiplicand_1(i: usize) -> usize {
        Self::wires_per_op() * i + 1
    }
    pub fn wire_ith_add_1(i: usize) -> usize {
        Self::wires_per_op() * i + 2
    }
    pub fn wire_ith_add_2(i: usize) -> usize {
        Self::wires_per_op() * i + 3
    }
    pub fn wire_ith_addend(i: usize) -> usize {
        Self::wires_per_op() * i + 4
    }
    pub fn wire_ith_output(i: usize) -> usize {
        Self::wires_per_op() * i + 5
    }
}

impl<F: RichField + Extendable<D>, const D: usize> Gate<F, D> for SimpleMulAddTestGate {
    fn id(&self) -> String {
        format!("{self:?}")
    }

    fn serialize(&self, dst: &mut Vec<u8>, _common_data: &CommonCircuitData<F, D>) -> IoResult<()> {
        dst.write_usize(self.num_ops)
    }

    fn deserialize(src: &mut Buffer, _common_data: &CommonCircuitData<F, D>) -> IoResult<Self> {
        let num_ops = src.read_usize()?;
        Ok(Self { num_ops })
    }

    fn eval_unfiltered(&self, vars: EvaluationVars<F, D>) -> Vec<F::Extension> {
        let mut constraints = Vec::with_capacity(self.num_ops);
        for i in 0..self.num_ops {
            let multiplicand_0 = vars.local_wires[Self::wire_ith_multiplicand_0(i)];
            let multiplicand_1 = vars.local_wires[Self::wire_ith_multiplicand_1(i)];
            let add_1 = vars.local_wires[Self::wire_ith_add_1(i)];
            let add_2 = vars.local_wires[Self::wire_ith_add_2(i)];
            let addend = vars.local_wires[Self::wire_ith_addend(i)];
            let output = vars.local_wires[Self::wire_ith_output(i)];

            let computed_output = multiplicand_0 * multiplicand_1 + add_1 + add_2 + addend;

            constraints.push(output - computed_output);
        }

        constraints
    }

    fn eval_unfiltered_base_one(
        &self,
        _vars: EvaluationVarsBase<F>,
        _yield_constr: StridedConstraintConsumer<F>,
    ) {
        panic!("use eval_unfiltered_base_packed instead");
    }

    fn eval_unfiltered_base_batch(&self, vars_base: EvaluationVarsBaseBatch<F>) -> Vec<F> {
        self.eval_unfiltered_base_batch_packed(vars_base)
    }

    fn eval_unfiltered_circuit(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        vars: EvaluationTargets<D>,
    ) -> Vec<ExtensionTarget<D>> {

        let mut constraints = Vec::with_capacity(self.num_ops);
        for i in 0..self.num_ops {
            let multiplicand_0 = vars.local_wires[Self::wire_ith_multiplicand_0(i)];
            let multiplicand_1 = vars.local_wires[Self::wire_ith_multiplicand_1(i)];
            let add_1 = vars.local_wires[Self::wire_ith_add_1(i)];
            let add_2 = vars.local_wires[Self::wire_ith_add_2(i)];
            let addend = vars.local_wires[Self::wire_ith_addend(i)];
            let output = vars.local_wires[Self::wire_ith_output(i)];

            let computed_output = {
                let rst = builder.mul_extension(multiplicand_0, multiplicand_1);
                builder.add_many_extension([rst, add_1, add_2, addend])
            };

            let diff = builder.sub_extension(output, computed_output);
            constraints.push(diff);
        }

        constraints
    }

    fn generators(&self, row: usize, _local_constants: &[F]) -> Vec<WitnessGeneratorRef<F, D>> {
        (0..self.num_ops)
            .map(|i| {
                WitnessGeneratorRef::new(
                    SimpleMulAddTestGenerator {
                        row,
                        i,
                        _phantom_data: PhantomData
                    }
                    .adapter(),
                )
            })
            .collect()
    }

    fn num_wires(&self) -> usize {
        self.num_ops * Self::wires_per_op()
    }

    fn num_constants(&self) -> usize {
        0
    }

    fn degree(&self) -> usize {
        3
    }

    fn num_constraints(&self) -> usize {
        self.num_ops
    }
}

impl<F: RichField + Extendable<D>, const D: usize> PackedEvaluableBase<F, D> for SimpleMulAddTestGate {
    fn eval_unfiltered_base_packed<P: PackedField<Scalar = F>>(
        &self,
        vars: EvaluationVarsBasePacked<P>,
        mut yield_constr: StridedConstraintConsumer<P>,
    ) {
        for i in 0..self.num_ops {
            let multiplicand_0 = vars.local_wires[Self::wire_ith_multiplicand_0(i)];
            let multiplicand_1 = vars.local_wires[Self::wire_ith_multiplicand_1(i)];
            let add_1 = vars.local_wires[Self::wire_ith_add_1(i)];
            let add_2 = vars.local_wires[Self::wire_ith_add_2(i)];
            let addend = vars.local_wires[Self::wire_ith_addend(i)];
            let output = vars.local_wires[Self::wire_ith_output(i)];

            let computed_output = multiplicand_0 * multiplicand_1 + add_1 + add_2 + addend;

            yield_constr.one(output - computed_output);
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct SimpleMulAddTestGenerator<F: RichField + Extendable<D>, const D: usize> {
    row: usize,
    i: usize,
    _phantom_data: PhantomData<F>
}

impl<F: RichField + Extendable<D>, const D: usize> SimpleGenerator<F, D>
    for SimpleMulAddTestGenerator<F, D> {

    fn id(&self) -> String {
        format!("{self:?}")
    }

    fn dependencies(&self) -> Vec<Target> {
        [
            SimpleMulAddTestGate::wire_ith_multiplicand_0(self.i),
            SimpleMulAddTestGate::wire_ith_multiplicand_1(self.i),
            SimpleMulAddTestGate::wire_ith_add_1(self.i),
            SimpleMulAddTestGate::wire_ith_add_2(self.i),
            SimpleMulAddTestGate::wire_ith_addend(self.i),
        ]
        .iter()
        .map(|&i| Target::wire(self.row, i))
        .collect()
    }

    fn run_once(&self, witness: &PartitionWitness<F>, out_buffer: &mut GeneratedValues<F>) {
        let get_wire = |wire: usize| -> F { witness.get_target(Target::wire(self.row, wire)) };

        let multiplicand_0 = get_wire(SimpleMulAddTestGate::wire_ith_multiplicand_0(self.i));
        let multiplicand_1 = get_wire(SimpleMulAddTestGate::wire_ith_multiplicand_1(self.i));
        let add_1 = get_wire(SimpleMulAddTestGate::wire_ith_add_1(self.i));
        let add_2 = get_wire(SimpleMulAddTestGate::wire_ith_add_2(self.i));
        let addend = get_wire(SimpleMulAddTestGate::wire_ith_addend(self.i));

        let output_target = Target::wire(self.row, SimpleMulAddTestGate::wire_ith_output(self.i));

        let computed_output =
            multiplicand_0 * multiplicand_1 + add_1 + add_2 + addend;

        out_buffer.set_target(output_target, computed_output)
    }

    fn serialize(&self, dst: &mut Vec<u8>, _common_data: &CommonCircuitData<F, D>) -> IoResult<()> {
        dst.write_usize(self.row)?;
        dst.write_usize(self.i)
    }

    fn deserialize(src: &mut Buffer, _common_data: &CommonCircuitData<F, D>) -> IoResult<Self> {
        let row = src.read_usize()?;
        let i = src.read_usize()?;
        Ok(Self {
            row,
            i,
            _phantom_data: PhantomData
        })
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    use anyhow::Result;
    use log::{LevelFilter, info};

    use plonky2_field::types::{Sample, Field};
    use plonky2::iop::witness::PartialWitness;
    use plonky2::gates::gate_testing::{test_eval_fns, test_low_degree};
    use plonky2::field::goldilocks_field::GoldilocksField;
    use plonky2::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};

    #[test]
    fn targets_test() {
        let mut log_builder = env_logger::Builder::from_default_env();
        log_builder.format_timestamp(None);
        log_builder.filter_level(LevelFilter::Info);
        log_builder.try_init().unwrap();

        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;

        let config = CircuitConfig::standard_recursion_config();

        let mut builder = CircuitBuilder::<F, D>::new(config);

        let x = builder.add_virtual_target();
        let y = builder.add_virtual_target();
        let xx = builder.add_virtual_target();
        let yy = builder.add_virtual_target();
        let z = builder.add_virtual_target();
        let oxy = builder.mul(x, y);
        let oxy = builder.add(oxy, xx);
        let oxy = builder.add(oxy, yy);
        let output = builder.add(oxy, z);
        // let x_add_x = builder.add(x, x);
        // let x_add_x_2 = builder.add(x, x);
        // let add_mul_add = builder.mul(x_add_x, x_add_x_2);


        info!("before build row `x`: {:?}", x);
        info!("before build row `y`: {:?}", y);
        info!("before build row `z`: {:?}", z);
        info!("before build row `oxy`: {:?}", oxy);
        info!("before build row `output`: {:?}", output);

        let x_value = F::rand();

        let data = builder.build::<C>();
        info!("degree: {}", data.common.constraint_degree());
        info!("the number of gates is: {}", data.common.gates.len());

        for g in &data.common.gates {
            info!("gate: {}", g.0.id());
        }

        let mut pw = PartialWitness::new();
        pw.set_target(x, x_value);
        pw.set_target(xx, x_value);
        pw.set_target(y, F::ONE);
        pw.set_target(yy, F::ONE);
        pw.set_target(z, F::ONE);

        let _proof = data.prove(pw.clone()).unwrap();

        // info!("after build and prove row `x`: {:?}", x);
        // info!("after build and prove row `x_add_x`: {:?}", x_add_x);
        // info!("before build row `x_add_x`: {:?}", x_add_x_2);
        // info!("after build and prove row `x_mul_x`: {:?}", add_mul_add);
    }

    #[test]
    fn low_degree() {
        let gate = SimpleMulAddTestGate::new_from_config(&CircuitConfig::standard_recursion_config());
        test_low_degree::<GoldilocksField, _, 4>(gate);
    }

    #[test]
    fn eval_fns() -> Result<()> {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        let gate = SimpleMulAddTestGate::new_from_config(&CircuitConfig::standard_recursion_config());
        test_eval_fns::<F, C, _, D>(gate)
    }
}
