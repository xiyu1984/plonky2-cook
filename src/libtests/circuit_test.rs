#[cfg(test)]
mod tests {
    use anyhow::Result;
    use log::{info, LevelFilter};

    use plonky2::iop::witness::{PartialWitness, WitnessWrite};
    use plonky2::plonk::circuit_builder::CircuitBuilder;
    use plonky2::plonk::circuit_data::CircuitConfig;
    use plonky2::hash::hash_types::RichField;
    use plonky2::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};

    use plonky2_field::extension::Extendable;
    use plonky2_field::types::Field;

    use plonky2_ecdsa::gadgets::recursive_proof::ProofTuple;

    fn make_array_sum<
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
        const D: usize,
    >(inputs: &Vec<F>) -> Result<ProofTuple<F, C, D>> {
        let config = CircuitConfig::standard_recursion_config();
        let mut builder = CircuitBuilder::<F, D>::new(config);

        let mut input_targets = Vec::new();
        for _ in 0..inputs.len() {
            input_targets.push(builder.add_virtual_target());
        }

        let sum_target = builder.add_many(&input_targets);

        let data = builder.build::<C>();

        let mut pw = PartialWitness::new();
        let mut sum = F::from_canonical_u64(0);
        for i in 0..inputs.len() {
            pw.set_target(input_targets[i], inputs[i]);
            sum += inputs[i];
        }

        pw.set_target(sum_target, sum);

        let proof = data.prove(pw).unwrap();
        data.verify(proof.clone()).expect("verify error");
        Ok((proof, data.verifier_only, data.common))
    }

    #[test]
    fn test_circuit_data() -> Result<()> {
        let mut log_builder = env_logger::Builder::from_default_env();
        log_builder.format_timestamp(None);
        log_builder.filter_level(LevelFilter::Info);
        let _ = log_builder.try_init();

        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;

        let res_1 = make_array_sum::<F, C, D>(&vec![F::from_canonical_u64(1), F::from_canonical_u64(2), F::from_canonical_u64(3)]).unwrap();

        let res_2 = make_array_sum::<F, C, D>(&vec![F::from_canonical_u64(1), F::from_canonical_u64(2)]).unwrap();

        info!("proof 1: {:?}, verifier data 1: {:?}, common data 1: {:?}", res_1.0.public_inputs, 1, 1);
        info!("proof 2: {:?}, verifier data 2: {:?}, common data 2: {:?}", res_2.0.public_inputs, 2, 2);
        // let common_data_1_bytes = common_data1.to_bytes(gate_serializer)

        Ok(())
    }
}