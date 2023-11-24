#[cfg(test)]
mod tests {
    use anyhow::Result;

    use log::{info, LevelFilter};
    use sha3::{Digest, Keccak256};

    use plonky2::field::extension::Extendable;
    use plonky2_field::types::{Field, PrimeField64};
    use plonky2::plonk::config::{GenericConfig, PoseidonGoldilocksConfig, KeccakGoldilocksConfig, Hasher, GenericHashOut};
    use plonky2::hash::{merkle_tree, merkle_proofs};
    use plonky2::hash::hash_types::{RichField, BytesHash};

    fn random_data<F: RichField>(n: usize, k: usize) -> Vec<Vec<F>> {
        (0..n).map(|_| F::rand_vec(k)).collect()
    }

    fn verify_all_leaves<
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
        const D: usize,
    >(
        leaves: Vec<Vec<F>>,
        cap_height: usize,
    ) -> Result<()> {
        let tree = merkle_tree::MerkleTree::<F, C::Hasher>::new(leaves.clone(), cap_height);
        for (i, leaf) in leaves.into_iter().enumerate() {
            let proof = tree.prove(i);
            merkle_proofs::verify_merkle_proof_to_cap(leaf, i, &tree.cap, &proof)?;
        }
        Ok(())
    }

    #[test]
    #[should_panic]
    fn test_cap_height_too_big() {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;

        let log_n = 8;
        let cap_height = log_n + 1; // Should panic if `cap_height > len_n`.

        let leaves = random_data::<F>(1 << log_n, 7);
        let _ = merkle_tree::MerkleTree::<F, <C as GenericConfig<D>>::Hasher>::new(leaves, cap_height);
    }

    #[test]
    fn test_cap_height_eq_log2_len() -> Result<()> {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;

        let log_n = 8;
        let n = 1 << log_n;
        let leaves = random_data::<F>(n, 7);

        verify_all_leaves::<F, C, D>(leaves, log_n)?;

        Ok(())
    }

    #[test]
    fn test_merkle_trees() -> Result<()> {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;

        let log_n = 8;
        let n = 1 << log_n;
        let leaves = random_data::<F>(n, 7);

        verify_all_leaves::<F, C, D>(leaves, 1)?;

        Ok(())
    }

    #[test]
    fn test_merkle_trees_keccak() -> Result<()> {
        const D: usize = 2;
        type C = KeccakGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;

        let log_n = 8;
        let n = 1 << log_n;
        let leaves = random_data::<F>(n, 7);

        verify_all_leaves::<F, C, D>(leaves, 1)?;

        Ok(())
    }

    #[test]
    fn test_two_keccak256_lib() -> Result<()> {
        let mut log_builder = env_logger::Builder::from_default_env();
        log_builder.format_timestamp(None);
        log_builder.filter_level(LevelFilter::Info);
        let _ = log_builder.try_init();

        const D: usize = 2;
        type C = KeccakGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;

        let src_buffer = F::from_canonical_u64(7);
        let k_25_out = {
            let k_25_hash = <C as GenericConfig<D>>::Hasher::hash_no_pad(vec![src_buffer].as_slice());
            <BytesHash<25> as GenericHashOut<F>>::to_bytes(&k_25_hash)
        };

        info!("The hash out by k_25 is: {:?}", hex::encode(k_25_out));

        let k_nomal_out = {
            let mut hasher = Keccak256::default();
            hasher.update(src_buffer.to_canonical_u64().to_le_bytes());
            println!("normal buffer: {:?}", src_buffer.to_canonical_u64().to_le_bytes());
            hasher.finalize()
        };

        info!("The keccak 256 hash of the message is: {:?}", hex::encode(k_nomal_out));

        Ok(())
    }

    #[test]
    fn details_of_merkle_tree_keccak() -> Result<()> {
        let mut log_builder = env_logger::Builder::from_default_env();
        log_builder.format_timestamp(None);
        log_builder.filter_level(LevelFilter::Info);
        let _ = log_builder.try_init();

        const D: usize = 2;
        type C = KeccakGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;

        let log_n = 1;
        let n = 1 << log_n;
        let mut leaves = vec![];

        for i in 0..n {
            leaves.push(vec![F::from_canonical_u64(i)]);
        }

        let tree = merkle_tree::MerkleTree::<F, <C as GenericConfig<D>>::Hasher>::new(leaves.clone(), 0);

        info!("The root hash of the merkle tree is: {:?}", tree.cap.0[0]);

        Ok(())
    }

}