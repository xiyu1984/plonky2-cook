#![allow(incomplete_features)]

// use log::{info, Level};
// use core::marker::PhantomData;

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

// This example is trivial but shows how to design and use `gate`
#[derive(Debug, Clone, Default)]
pub struct SimpleExpConstantGate {
    pub num_limbs: usize
}

impl SimpleExpConstantGate {
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

    /// The `i`th power of the exponent.
    pub fn wire_power_i(&self, i: usize) -> usize {
        debug_assert!(i <= self.num_limbs);
        debug_assert!(i > 0);
        i - 1
    }

    pub fn wire_output(&self) -> usize {
        self.num_limbs
    }
}

impl<F: RichField + Extendable<D>, const D: usize> Gate<F, D> for SimpleExpConstantGate {
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

        let power_0_wire: <F as Extendable<D>>::Extension = F::ONE.into();
        let v_base = vars.local_constants[0];

        let mut computed_cur_power = power_0_wire;

        let mut pre_wire_value = computed_cur_power;
        for i in 1..(self.num_limbs + 1) {
            computed_cur_power = pre_wire_value * v_base;

            let power_i_wire = vars.local_wires[self.wire_power_i(i)];

            constraints.push(power_i_wire - computed_cur_power);

            pre_wire_value = power_i_wire;
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

        let v_base = vars.local_constants[0];
        let mut computed_cur_power = builder.constant_extension(F::ONE.into());
        
        let mut pre_power = computed_cur_power;

        for i in 1..(self.num_limbs + 1) {
            computed_cur_power = builder.mul_extension(pre_power, v_base);

            let power_i_wire = vars.local_wires[self.wire_power_i(i)];

            constraints.push(builder.sub_extension(power_i_wire, computed_cur_power));

            pre_power = power_i_wire;
        }

        let output = vars.local_wires[self.wire_output()];
        constraints.push(builder.sub_extension(computed_cur_power, output));

        constraints
    }

    fn generators(&self, row: usize, local_constants: &[F]) -> Vec<WitnessGeneratorRef<F, D>> {
        let gen = SimpleExpConstantGenerator {
            row,
            gate: self.clone(),
            const_base: local_constants[0]
        };

        vec![WitnessGeneratorRef::new(gen.adapter())]
    }

    // `num_limbs` for powers, 1 for `output`
    fn num_wires(&self) -> usize {
        self.num_limbs + 1
    }

    fn num_constants(&self) -> usize {
        1
    }

    // how to determine the `degree`?
    // the `degree` is determined by the whole connected circuits of the gate
    // Notice: the degree here is different from `gate_with_veriable_vars` as every limb is decomposed from the calculation chain, 
    // which is shown in the `for` loop of the function `eval_unfiltered_circuit`
    fn degree(&self) -> usize {
        2
    }

    fn num_constraints(&self) -> usize {
        self.num_limbs + 1
    }
}

impl<F: RichField + Extendable<D>, const D: usize> PackedEvaluableBase<F, D> for SimpleExpConstantGate {
    fn eval_unfiltered_base_packed<P: PackedField<Scalar = F>>(
        &self,
        vars: EvaluationVarsBasePacked<P>,
        mut yield_constr: StridedConstraintConsumer<P>,
    ) {

        let mut computed_cur_power: P = F::ONE.into();
        let v_base = vars.local_constants[0];

        let mut pre_wire_value = computed_cur_power;
        for i in 1..(self.num_limbs + 1) {
            computed_cur_power = pre_wire_value * v_base;

            let power_i_wire = vars.local_wires[self.wire_power_i(i)];

            yield_constr.one(power_i_wire - computed_cur_power);

            pre_wire_value = power_i_wire;
        }

        let output = vars.local_wires[self.wire_output()];
        yield_constr.one(computed_cur_power - output);
    }
}

#[derive(Clone, Debug, Default)]
pub struct SimpleExpConstantGenerator<F: RichField + Extendable<D>, const D: usize> {
    pub row: usize,
    pub gate: SimpleExpConstantGate,
    pub const_base: F
}

impl<F: RichField + Extendable<D>, const D: usize> SimpleGenerator<F, D>
    for SimpleExpConstantGenerator<F, D> {

    fn id(&self) -> String {
        format!("{self:?}")
    }

    fn dependencies(&self) -> Vec<Target> {
        let local_target = |column: usize| -> Target {
            Target::wire(self.row, column)
        };

        let mut deps = Vec::with_capacity(self.gate.num_limbs);

        for i in 1..(self.gate.num_limbs + 1) {
            deps.push(local_target(self.gate.wire_power_i(i)));
        }

        deps
    }

    fn run_once(&self, witness: &PartitionWitness<F>, out_buffer: &mut GeneratedValues<F>) {
        let get_wire_target = |column: usize| -> Target { 
            Target::wire(self.row, column)
        };

        let get_wire_value = |wire: usize| -> F { witness.get_target(Target::wire(self.row, wire)) };

        let base = self.const_base;
        // let mut computed_output = self.const_0;
        let mut computed_output = F::ONE;

        let mut pre_power_value = computed_output;
        for i in 1..(self.gate.num_limbs + 1) {
            computed_output = pre_power_value * base;

            out_buffer.set_target(get_wire_target(self.gate.wire_power_i(i)), computed_output);

            pre_power_value = get_wire_value(self.gate.wire_power_i(i));

            // println!("In `run_once`, row {} column {}, value {}", self.row, self.gate.wire_power_i(i), pre_power_value);
        }

        let output_target = get_wire_target(self.gate.wire_output());

        // println!("In `run_once output`, row {} column {}, value {}", self.row, self.gate.wire_output(), computed_output);

        out_buffer.set_target(output_target, computed_output)
    }

    fn serialize(&self, dst: &mut Vec<u8>, _common_data: &CommonCircuitData<F, D>) -> IoResult<()> {
        dst.write_usize(self.row)?;
        self.gate.serialize(dst, _common_data)?;
        dst.write_field(self.const_base)
    }

    fn deserialize(src: &mut Buffer, _common_data: &CommonCircuitData<F, D>) -> IoResult<Self> {
        let row = src.read_usize()?;
        let gate = SimpleExpConstantGate::deserialize(src, _common_data)?;
        let const_base = src.read_field()?;
        // Ok(Self { row, gate, const_0 })
        Ok(Self {row, gate, const_base})
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    use anyhow::Result;

    use plonky2::gates::gate_testing::{test_eval_fns, test_low_degree};
    use plonky2::field::goldilocks_field::GoldilocksField;
    // use plonky2::field::types::{PrimeField, Sample};
    use plonky2::field::types::Field;
    use plonky2::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};
    use plonky2::iop::witness::{PartialWitness, WitnessWrite};

    #[test]
    fn low_degree() {
        // Here we can set the input parameter `power` very large, for example `16000` just needs abount 2s.
        // This is because the calculation chain is decomposed.
        let gate = SimpleExpConstantGate::new(16, &CircuitConfig::standard_recursion_config());
        test_low_degree::<GoldilocksField, _, 4>(gate);
    }

    #[test]
    fn eval_fns() -> Result<()> {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        let gate = SimpleExpConstantGate::new(8, &CircuitConfig::standard_recursion_config());
        test_eval_fns::<F, C, _, D>(gate)
    }

    // this will be failed
    // Note:  `generator` cannot be used directly when using `gate`
    #[test]
    #[should_panic]
    fn test_generator() {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;

        let config = CircuitConfig::standard_recursion_config();
        let mut builder = CircuitBuilder::<F, D>::new(config);

        let secg = SimpleExpConstantGenerator::<F, D> {
            row: 1,     // No matter how to set the row directly, the underlying error will happen
            gate: SimpleExpConstantGate { num_limbs: 0 },
            const_base: F::from_canonical_u32(2)
        };

        builder.add_simple_generator(secg.clone());

        let get_wire_target = |column: usize| -> Target { 
            Target::wire(secg.row, column)
        };

        // for i in 1..(secg.gate.num_limbs + 1) {
        //     println!("Target power {}: {:?}", i, get_wire_target(secg.gate.wire_power_i(i)));
        // }

        // println!("Target output: {:?}", get_wire_target(secg.gate.wire_output()));

        let mut pw = PartialWitness::new();

        let mut powers = F::ONE;
        for i in 1..(secg.gate.num_limbs + 1) {
            powers *= secg.const_base;
            pw.set_target(get_wire_target(secg.gate.wire_power_i(i)), powers);

            // println!("In `test_generator set pw`, row {} column {}, value {}", secg.row, secg.gate.wire_power_i(i), powers);
        }

        pw.set_target(get_wire_target(secg.gate.wire_output()), powers);
        // println!("In `test_generator output`, row {} column {}, value {}", secg.row,secg.gate.wire_output(), powers);

        // println!("in pw: {:?}", pw.get_target(Target::wire(1, 0)));

        let data = builder.build::<C>();
        let proof = data.prove(pw.clone()).unwrap();

        data.verify(proof).unwrap();

    }

    // Nice trying! This is how to use `gate`.
    // Here is the correct way to use `gate`.
    #[test]
    fn test_gate() -> Result<()> {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;

        let config = CircuitConfig::standard_recursion_config();
        let mut builder = CircuitBuilder::<F, D>::new(config.clone());

        let base = F::from_canonical_u32(2);
        let gate = SimpleExpConstantGate::new(8, &config);
        let row = builder.add_gate(gate.clone(), vec![base]);

        let get_wire_target = |column: usize| -> Target { 
            Target::wire(row, column)
        };

        let mut pw = PartialWitness::new();

        let mut powers = F::ONE;
        for i in 1..(gate.num_limbs + 1) {
            powers *= base;
            pw.set_target(get_wire_target(gate.wire_power_i(i)), powers);

            // println!("In `test_generator set pw`, row {} column {}, value {}", row, gate.wire_power_i(i), powers);
        }

        pw.set_target(get_wire_target(gate.wire_output()), powers);
        // println!("In `test_generator output`, row {} column {}, value {}", row, gate.wire_output(), powers);

        let circuit = builder.build::<C>();
        let proof = circuit.prove(pw.clone())?;

        circuit.verify(proof)

        // Ok(())
    }
}
