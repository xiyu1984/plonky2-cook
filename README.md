# Cook

## Description

This repo shows how to use `generator` and `gate`, which are the low level API of `plonky2`.  

## Env

### Rust 

```sh
rustup override set nightly
```
### plonky2

This repo uses the latest [`plonky2` version](https://github.com/0xPolygonZero/plonky2).  

## Test

- Test single

    ```sh
    cargo test -r --package plonky2-cook --lib -- gates::simple_add_gate::tests::targets_test --exact --nocapture
    ```

- Test lib `simple_add_gates`

    ```sh
    cargo test -r --package plonky2-cook --lib -- -- gates::simple_add_gate::tests --nocapture
    ```

    - Test single: `targets_test`

        ```sh
        cargo test -r --package plonky2-cook --lib -- gates::simple_add_gate::tests::targets_test --exact --nocapture
        ```

- Test lib `gate_with_veriable_vars`

    ```sh
    cargo test -r --package plonky2-cook --lib -- gates::gate_with_veriable_vars::tests --nocapture
    ```

- Test lib `g_w_v_v_constant`

    ```sh
    cargo test -r --package plonky2-cook --lib -- gates::g_w_v_v_constant::tests --nocapture

    cargo test -r --package plonky2-cook --lib -- gates::g_w_v_v_constant::tests::test_generator --exact --nocapture

    cargo test -r --package plonky2-cook --lib -- gates::g_w_v_v_constant::tests::test_gate --exact --nocapture
    ```

- `libtests`
    - `merkle_tree_test`

        ```sh
        cargo test -r --package plonky2-cook --lib -- libtests::merkle_tree_test::tests --nocapture

        cargo test -r --package plonky2-cook --lib -- libtests::merkle_tree_test::tests::details_of_merkle_tree_keccak --exact --nocapture

        cargo test -r --package plonky2-cook --lib -- libtests::merkle_tree_test::tests::test_two_keccak256_lib --exact --nocapture

        ```

- Test all

    ```sh
    cargo test -r
    ```
