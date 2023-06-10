#[cfg(test)]
mod tests {
    use plonky2::{
        field::{goldilocks_field::GoldilocksField, types::Field},
        iop::{
            target::Target,
            witness::{PartialWitness, WitnessWrite},
        },
        plonk::{
            circuit_builder::CircuitBuilder,
            circuit_data::{CircuitConfig, CircuitData},
            config::PoseidonGoldilocksConfig,
            proof::ProofWithPublicInputs,
        },
    };

    // 利用する有限体
    type F = GoldilocksField;
    // 2次の拡大体を利用するという設定
    const D: usize = 2;
    // ハッシュに Poseidon を利用し proof を作る
    type C = PoseidonGoldilocksConfig;

    #[test]
    fn test_plonky2_add() {
        // 回路のサイズや各種設定が入る構造体
        let config = CircuitConfig::standard_recursion_config();
        // 回路の制約を扱う
        let mut builder = CircuitBuilder::<F, D>::new(config);

        // GOldilocksField 上の 1
        let one = F::from_canonical_u64(1);
        // あるいは let one = F::ONE;
        let two = F::TWO;

        // 回路上の(空の)変数（ワイヤ）を定義する
        // まだ位置がわからないので、virtual target となる => 最終的に表の中の位置が固定され、 Wire になる
        let a = builder.add_virtual_target();
        let b = builder.add_virtual_target();
        // c <== a + b という制約を課す
        let c = builder.add(a, b);

        // ターゲットに witness(値)を割り当てる
        // PartialWitness は target - witness 間の関係を管理する構造体
        let mut pw = PartialWitness::<F>::new();

        // a <== one
        pw.set_target(a, one);
        // b <== one
        pw.set_target(b, one);
        // c <== one + one
        pw.set_target(c, two);

        // circuit の build
        let data = builder.build::<C>();

        // proof 生成、仮に invalid な条件の場合はここで失敗する
        let proof = data.prove(pw).unwrap();

        // proof の検証
        data.verify(proof).unwrap();
    }

    struct InnerTarget {
        a: Target,
        b: Target,
        c: Target,
    }

    // inner circuit (再帰証明の対象になる回路)
    fn build_inner_circuit() -> (CircuitData<F, C, D>, InnerTarget) {
        let config = CircuitConfig::standard_recursion_config();
        let mut builder = CircuitBuilder::<F, D>::new(config);

        let one = F::ONE;

        let a = builder.add_virtual_target();
        let b = builder.add_virtual_target();
        let c = builder.add(a, b);

        let mut pw = PartialWitness::<F>::new();
        pw.set_target(a, one);
        pw.set_target(b, one);
        pw.set_target(c, F::TWO);

        // circuit の build
        let data = builder.build::<C>();
        let target = InnerTarget { a, b, c };
        (data, target)
    }

    fn generate_inner_proof(
        data: &CircuitData<F, C, D>,
        it: &InnerTarget,
    ) -> ProofWithPublicInputs<F, C, D> {
        let mut pw = PartialWitness::new();
        pw.set_target(it.a, F::ONE);
        pw.set_target(it.b, F::TWO);
        pw.set_target(it.c, F::from_canonical_u64(3));
        // proof の生成
        data.prove(pw).unwrap()
    }

    #[test]
    fn test_recursive_proof() {
        let (inner_data, inner_target) = build_inner_circuit();

        let config = CircuitConfig::standard_recursion_config();
        let mut builder = CircuitBuilder::<F, D>::new(config);

        // recuirsive proof
        // proof_with_pis という virtual target を検証する制約を回路に追加
        let inner_verifier_data = builder.constant_verifier_data(&inner_data.verifier_only);
        let proof_with_pis = builder.add_virtual_proof_with_pis(&inner_data.common);
        builder.verify_proof::<C>(&proof_with_pis, &inner_verifier_data, &inner_data.common);

        // inner_proof の作成
        let inner_proof = generate_inner_proof(&inner_data, &inner_target);

        // witness の割当
        let mut pw = PartialWitness::<F>::new();
        // proof_with_pis に値を割り当てる
        pw.set_proof_with_pis_target(&proof_with_pis, &inner_proof);

        let data = builder.build::<C>();
        let proof = data.prove(pw).unwrap();
        data.verify(proof).unwrap();
    }
}
