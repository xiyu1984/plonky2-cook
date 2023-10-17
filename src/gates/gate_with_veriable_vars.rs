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

#[derive(Debug, Clone, Default)]
pub struct SimpleExpTestGate {
    pub num_limbs: usize
}

impl SimpleExpTestGate {
    pub fn new(power: usize, config: &CircuitConfig) -> Self {
        debug_assert!(power < Self::max_power(config));

        Self {
            num_limbs: power,
        }
    }

    /// Determine the maximum number of operations that can fit in one gate for the given config.
    fn max_power(config: &CircuitConfig) -> usize {
        // 3 wires are reserved for the 0 power, base and output.
        let max_for_routed_wires = config.num_routed_wires - 3;
        let max_for_wires = (config.num_wires - 3) / 2;
        max_for_routed_wires.min(max_for_wires)
    }

    // the value of the first target must be `1`
    pub fn wire_base(&self) -> usize {
        0
    }

    /// The `i`th power of the exponent.
    pub fn wire_power_i(&self, i: usize) -> usize {
        debug_assert!(i <= self.num_limbs);
        debug_assert!(i > 0);

        i
    }

    pub fn wire_output(&self) -> usize {
        self.num_limbs + 1
    }
}

impl<F: RichField + Extendable<D>, const D: usize> Gate<F, D> for SimpleExpTestGate {
    fn id(&self) -> String {
        format!("{self:?}")
    }

    fn serialize(&self, dst: &mut Vec<u8>, _common_data: &CommonCircuitData<F, D>) -> IoResult<()> {
        dst.write_usize(self.num_limbs)
    }

    fn deserialize(src: &mut Buffer, _common_data: &CommonCircuitData<F, D>) -> IoResult<Self> {
        let num_limbs = src.read_usize()?;
        Ok(Self { num_limbs })
    }

    fn eval_unfiltered(&self, vars: EvaluationVars<F, D>) -> Vec<F::Extension> {

        let mut constraints = Vec::with_capacity(self.num_limbs + 1);

        let power_0_wire = F::ONE.into();
        // let power_0_wire = vars.local_constants[0];
        // constraints.push(power_0_wire - F::ONE.into());

        let mut computed_cur_power = power_0_wire;
        let v_base = vars.local_wires[self.wire_base()];

        for i in 1..(self.num_limbs + 1) {
            computed_cur_power *= v_base;

            let power_i_wire = vars.local_wires[self.wire_power_i(i)];

            constraints.push(power_i_wire - computed_cur_power);
        }

        let output = vars.local_wires[self.wire_output()];
        constraints.push(computed_cur_power - output);

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

        let mut constraints = Vec::with_capacity(self.num_limbs + 2);

        // let power_0_wire = vars.local_constants[0];
        // let const_0_target = builder.constant_extension(F::ONE.into());
        // constraints.push(builder.sub_extension(power_0_wire, const_0_target));

        // let mut computed_cur_power = power_0_wire;
        let mut computed_cur_power = builder.constant_extension(F::ONE.into());
        let v_base = vars.local_wires[self.wire_base()];

        for i in 1..(self.num_limbs + 1) {
            computed_cur_power = builder.mul_extension(computed_cur_power, v_base);

            let power_i_wire = vars.local_wires[self.wire_power_i(i)];

            constraints.push(builder.sub_extension(power_i_wire, computed_cur_power));
        }

        let output = vars.local_wires[self.wire_output()];
        constraints.push(builder.sub_extension(computed_cur_power, output));

        constraints
    }

    fn generators(&self, row: usize, _local_constants: &[F]) -> Vec<WitnessGeneratorRef<F, D>> {
        let gen = SimpleExpTestGenerator {
            row,
            gate: self.clone(),
            // const_0: F::ONE
            _phatom: PhantomData
        };

        vec![WitnessGeneratorRef::new(gen.adapter())]
    }

    // 1 for `base`, `num_limbs` for powers, 1 for `output`
    fn num_wires(&self) -> usize {
        self.num_limbs + 2
    }

    fn num_constants(&self) -> usize {
        0
    }

    // how to determine the `degree`?
    // the `degree` is determined by the whole connected circuits of the gate
    fn degree(&self) -> usize {
        self.num_limbs
    }

    fn num_constraints(&self) -> usize {
        self.num_limbs + 1
    }
}

impl<F: RichField + Extendable<D>, const D: usize> PackedEvaluableBase<F, D> for SimpleExpTestGate {
    fn eval_unfiltered_base_packed<P: PackedField<Scalar = F>>(
        &self,
        vars: EvaluationVarsBasePacked<P>,
        mut yield_constr: StridedConstraintConsumer<P>,
    ) {
        let power_0_wire: P = F::ONE.into();
        // let power_0_wire = vars.local_constants[0];
        // yield_constr.one(power_0_wire - F::ONE);

        let mut computed_cur_power = power_0_wire;
        let v_base = vars.local_wires[self.wire_base()];

        for i in 1..(self.num_limbs + 1) {
            computed_cur_power *= v_base;

            let power_i_wire = vars.local_wires[self.wire_power_i(i)];

            yield_constr.one(power_i_wire - computed_cur_power);
        }

        let output = vars.local_wires[self.wire_output()];
        yield_constr.one(computed_cur_power - output);
    }
}

#[derive(Clone, Debug, Default)]
pub struct SimpleExpTestGenerator<F: RichField + Extendable<D>, const D: usize> {
    pub row: usize,
    pub gate: SimpleExpTestGate,
    // pub const_0: F
    pub _phatom: PhantomData<F>
}

impl<F: RichField + Extendable<D>, const D: usize> SimpleGenerator<F, D>
    for SimpleExpTestGenerator<F, D> {

    fn id(&self) -> String {
        format!("{self:?}")
    }

    fn dependencies(&self) -> Vec<Target> {
        let local_target = |column: usize| -> Target {
            Target::wire(self.row, column)
        };

        let mut deps = Vec::with_capacity(self.gate.num_limbs + 1);
        deps.push(local_target(self.gate.wire_base()));

        for i in 1..(self.gate.num_limbs + 1) {
            deps.push(local_target(i));
        }

        deps
    }

    fn run_once(&self, witness: &PartitionWitness<F>, out_buffer: &mut GeneratedValues<F>) {
        let get_wire_target = |column: usize| -> Target { 
            Target::wire(self.row, column)
        };

        let get_wire_value = |wire: usize| -> F { witness.get_target(Target::wire(self.row, wire)) };

        let base = get_wire_value(self.gate.wire_base());
        // let mut computed_output = self.const_0;
        let mut computed_output = F::ONE;

        for i in 1..(self.gate.num_limbs + 1) {
            computed_output *= base;
            out_buffer.set_target(get_wire_target(i), computed_output);
        }

        let output_target = get_wire_target(self.gate.wire_output());
        out_buffer.set_target(output_target, computed_output)
    }

    fn serialize(&self, dst: &mut Vec<u8>, _common_data: &CommonCircuitData<F, D>) -> IoResult<()> {
        dst.write_usize(self.row)?;
        self.gate.serialize(dst, _common_data)
        // dst.write_field(self.const_0)
    }

    fn deserialize(src: &mut Buffer, _common_data: &CommonCircuitData<F, D>) -> IoResult<Self> {
        let row = src.read_usize()?;
        let gate = SimpleExpTestGate::deserialize(src, _common_data)?;
        // let const_0 = src.read_field()?;
        // Ok(Self { row, gate, const_0 })
        Ok(Self {row, gate, _phatom: PhantomData})
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    use anyhow::Result;

    use plonky2::gates::gate_testing::{test_eval_fns, test_low_degree};
    use plonky2::field::goldilocks_field::GoldilocksField;
    use plonky2::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};

    #[test]
    fn low_degree() {
        let gate = SimpleExpTestGate::new(8, &CircuitConfig::standard_recursion_config());
        test_low_degree::<GoldilocksField, _, 4>(gate);
    }

    #[test]
    fn eval_fns() -> Result<()> {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        let gate = SimpleExpTestGate::new(8, &CircuitConfig::standard_recursion_config());
        test_eval_fns::<F, C, _, D>(gate)
    }
}
