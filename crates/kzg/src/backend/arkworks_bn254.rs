use crate::{
    errs::{KZGError, KZGResult},
    primitives::{
        poly::FpPolynomial,
        serde::{ark_deserialize, ark_serialize},
    },
    HomomorphicPolyComElem, PolyComScheme, ToBytes,
};
use ark_bn254::{Bn254, Fr, G1Projective};
use ark_ec::{pairing::Pairing, CurveGroup, PrimeGroup, VariableBaseMSM};
use ark_ff::{One, PrimeField};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize, Compress, Validate};
use ark_std::{ops::*, Zero};
use serde::{Deserialize, Serialize};

/// KZG commitment scheme over the `Group`.
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, Default)]
pub struct KZGCommitment<G: CanonicalSerialize + CanonicalDeserialize>(
    #[serde(serialize_with = "ark_serialize", deserialize_with = "ark_deserialize")] pub G,
);

impl<G> ToBytes for G
where
    G: CurveGroup,
{
    fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        self.serialize_with_mode(&mut buf, Compress::Yes).unwrap();
        buf
    }
}

impl<G> ToBytes for KZGCommitment<G>
where
    G: CurveGroup,
{
    fn to_bytes(&self) -> Vec<u8> {
        self.0.to_bytes()
    }
}

impl HomomorphicPolyComElem for KZGCommitment<G1Projective> {
    type Scalar = Fr;
    fn get_base() -> Self {
        KZGCommitment(G1Projective::generator())
    }

    fn get_identity() -> Self {
        KZGCommitment(G1Projective::zero())
    }

    fn add(&self, other: &Self) -> Self {
        KZGCommitment(self.0.add(&other.0))
    }

    fn add_assign(&mut self, other: &Self) {
        self.0.add_assign(&other.0)
    }

    fn sub(&self, other: &Self) -> Self {
        KZGCommitment(self.0.sub(&other.0))
    }

    fn sub_assign(&mut self, other: &Self) {
        self.0.sub_assign(&other.0)
    }

    fn mul(&self, exp: &Fr) -> Self {
        KZGCommitment(self.0.mul(exp))
    }

    fn mul_assign(&mut self, exp: &Fr) {
        self.0.mul_assign(exp)
    }
}

impl<F: PrimeField> ToBytes for FpPolynomial<F> {
    fn to_bytes(&self) -> Vec<u8> {
        unimplemented!()
    }
}

impl<F: PrimeField> HomomorphicPolyComElem for FpPolynomial<F> {
    type Scalar = F;

    fn get_base() -> Self {
        unimplemented!()
    }

    fn get_identity() -> Self {
        unimplemented!()
    }

    fn add(&self, other: &Self) -> Self {
        self.add(other)
    }

    fn add_assign(&mut self, other: &Self) {
        self.add_assign(other)
    }

    fn sub(&self, other: &Self) -> Self {
        self.sub(other)
    }

    fn sub_assign(&mut self, other: &Self) {
        self.sub_assign(other)
    }

    fn mul(&self, exp: &F) -> Self {
        self.mul_scalar(exp)
    }

    fn mul_assign(&mut self, exp: &F) {
        self.mul_scalar_assign(exp)
    }
}

/// KZG opening proof.
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct KZGOpenProof<G1: CanonicalSerialize + CanonicalDeserialize>(
    #[serde(serialize_with = "ark_serialize", deserialize_with = "ark_deserialize")] pub G1,
);

impl<G: PrimeGroup> ToBytes for KZGOpenProof<G> {
    fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        self.0.serialize_with_mode(&mut buf, Compress::Yes).unwrap();
        buf
    }
}

/// KZG commitment scheme about `PairingEngine`.
#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct KZGCommitmentScheme<P: Pairing> {
    /// public parameter about G1.
    #[serde(serialize_with = "ark_serialize", deserialize_with = "ark_deserialize")]
    pub public_parameter_group_1: Vec<P::G1>,
    /// public parameter about G1.
    #[serde(serialize_with = "ark_serialize", deserialize_with = "ark_deserialize")]
    pub public_parameter_group_2: Vec<P::G2>,
}

impl<P: Pairing> KZGCommitmentScheme<P> {
    /// Create a new instance of a KZG polynomial commitment scheme.
    /// `max_degree` - max degree of the polynomial,
    /// `prng` - pseudo-random generator.
    /// WARN only for testing purposes.
    #[cfg(test)]
    pub fn new<R: ark_std::rand::RngCore>(
        max_degree: usize,
        prng: &mut R,
    ) -> KZGCommitmentScheme<P> {
        use ark_std::UniformRand;
        let s = P::ScalarField::rand(prng);

        let mut public_parameter_group_1: Vec<P::G1> = Vec::new();

        let mut elem_g1 = P::G1::generator();

        for _ in 0..=max_degree {
            public_parameter_group_1.push(elem_g1.clone());
            elem_g1 = elem_g1.mul(&s);
        }

        let mut public_parameter_group_2: Vec<P::G2> = Vec::new();
        let elem_g2 = P::G2::generator();
        public_parameter_group_2.push(elem_g2.clone());
        public_parameter_group_2.push(elem_g2.mul(&s));

        KZGCommitmentScheme {
            public_parameter_group_1,
            public_parameter_group_2,
        }
    }

    /// Serialize the parameters to unchecked bytes.
    pub fn to_unchecked_bytes(&self) -> KZGResult<Vec<u8>> {
        let mut bytes = vec![];
        let len_1 = self.public_parameter_group_1.len() as u32;
        let len_2 = self.public_parameter_group_2.len() as u32;
        bytes.extend(len_1.to_le_bytes());
        bytes.extend(len_2.to_le_bytes());

        for i in &self.public_parameter_group_1 {
            let mut buf = Vec::new();
            i.serialize_with_mode(&mut buf, Compress::No).unwrap();
            bytes.extend(buf);
        }
        for i in &self.public_parameter_group_2 {
            let mut buf = Vec::new();
            i.serialize_with_mode(&mut buf, Compress::No).unwrap();
            bytes.extend(buf);
        }
        Ok(bytes)
    }

    /// Deserialize the parameters from unchecked bytes.
    pub fn from_unchecked_bytes(bytes: &[u8]) -> KZGResult<Self> {
        if bytes.len() < 8 {
            return Err(KZGError::DeserializationError);
        }
        let mut len_1_bytes = [0u8; 4];
        let mut len_2_bytes = [0u8; 4];
        len_1_bytes.copy_from_slice(&bytes[0..4]);
        len_2_bytes.copy_from_slice(&bytes[4..8]);
        let len_1 = u32::from_le_bytes(len_1_bytes) as usize;
        let len_2 = u32::from_le_bytes(len_2_bytes) as usize;
        let n_1 = P::G1::default().serialized_size(Compress::No);
        let n_2 = P::G2::default().serialized_size(Compress::No);

        let bytes_1 = &bytes[8..];
        let bytes_2 = &bytes[8 + (n_1 * len_1)..];
        let mut p1 = vec![];
        let mut p2 = vec![];

        for i in 0..len_1 {
            let reader = &bytes_1[n_1 * i..n_1 * (i + 1)];
            let g1 = P::G1::deserialize_with_mode(reader, Compress::No, Validate::No)
                .map_err(|_| KZGError::DeserializationError)?;
            p1.push(g1);
        }

        for i in 0..len_2 {
            let reader = &bytes_2[n_2 * i..n_2 * (i + 1)];
            let g2 = P::G2::deserialize_with_mode(reader, Compress::No, Validate::No)
                .map_err(|_| KZGError::DeserializationError)?;
            p2.push(g2);
        }

        Ok(Self {
            public_parameter_group_1: p1,
            public_parameter_group_2: p2,
        })
    }
}

/// KZG commitment scheme over the BN254 curve
pub type KZGCommitmentSchemeBN254 = KZGCommitmentScheme<Bn254>;

impl PolyComScheme for KZGCommitmentSchemeBN254 {
    type Field = Fr;
    type Commitment = KZGCommitment<G1Projective>;

    fn max_degree(&self) -> usize {
        self.public_parameter_group_1.len() - 1
    }

    fn commit(&self, polynomial: &FpPolynomial<Fr>) -> KZGResult<Self::Commitment> {
        let coefs = polynomial.get_coefs_ref();

        let degree = polynomial.degree();

        if degree + 1 > self.public_parameter_group_1.len() {
            return Err(KZGError::DegreeError);
        }

        let points_raw =
            G1Projective::normalize_batch(&self.public_parameter_group_1[0..degree + 1]);

        let commitment_value = G1Projective::msm(&points_raw, coefs).unwrap();

        Ok(KZGCommitment(commitment_value))
    }

    fn eval(&self, poly: &FpPolynomial<Self::Field>, point: &Self::Field) -> Self::Field {
        poly.eval(point)
    }

    fn apply_blind_factors(
        &self,
        commitment: &Self::Commitment,
        blinds: &[Self::Field],
        zeroing_degree: usize,
    ) -> Self::Commitment {
        let mut commitment = commitment.0;
        for (i, blind) in blinds.iter().enumerate() {
            let mut blind = *blind;
            commitment += &(self.public_parameter_group_1[i] * blind);
            blind = blind.neg();
            commitment += &(self.public_parameter_group_1[zeroing_degree + i] * blind);
        }
        KZGCommitment(commitment)
    }

    fn prove(
        &self,
        poly: &FpPolynomial<Self::Field>,
        x: &Self::Field,
        max_degree: usize,
    ) -> KZGResult<Self::Commitment> {
        let eval = poly.eval(x);

        if poly.degree() > max_degree {
            return Err(KZGError::DegreeError);
        }

        let nominator = poly.sub(&FpPolynomial::from_coefs(vec![eval]));

        // Negation must happen in Fq
        let point_neg = x.neg();

        // X - x
        let vanishing_poly = FpPolynomial::from_coefs(vec![point_neg, Self::Field::one()]);
        let (q_poly, r_poly) = nominator.div_rem(&vanishing_poly); // P(X)-P(x) / (X-x)

        if !r_poly.is_zero() {
            return Err(KZGError::PCSProveEvalError);
        }

        let proof = self.commit(&q_poly).unwrap();
        Ok(proof)
    }

    fn verify(
        &self,
        cm: &Self::Commitment,
        _degree: usize,
        point: &Self::Field,
        eval: &Self::Field,
        proof: &Self::Commitment,
    ) -> KZGResult<()> {
        let g1_0 = self.public_parameter_group_1[0];
        let g2_0 = self.public_parameter_group_2[0];
        let g2_1 = self.public_parameter_group_2[1];

        let x_minus_point_group_element_group_2 = &g2_1.sub(&g2_0.mul(point));

        let left_pairing_eval = if eval.is_zero() {
            Bn254::pairing(cm.0, g2_0)
        } else {
            Bn254::pairing(cm.0.sub(&g1_0.mul(eval)), g2_0)
        };

        let right_pairing_eval = Bn254::pairing(proof.0, x_minus_point_group_element_group_2);

        if left_pairing_eval == right_pairing_eval {
            Ok(())
        } else {
            Err(KZGError::PCSProveEvalError)
        }
    }

    fn shrink_to_verifier_only(&self) -> KZGResult<Self> {
        Ok(Self {
            public_parameter_group_1: vec![self.public_parameter_group_1[0]],
            public_parameter_group_2: vec![
                self.public_parameter_group_2[0],
                self.public_parameter_group_2[1],
            ],
        })
    }
}

#[cfg(test)]
mod tests_kzg_impl {
    use ark_std::test_rng;

    use super::*;

    fn check_public_parameters_generation<P: Pairing>() {
        let param_size = 5;
        let mut prng = test_rng();
        let kzg_scheme = KZGCommitmentScheme::<P>::new(param_size, &mut prng);
        let g1_power1 = kzg_scheme.public_parameter_group_1[1].clone();
        let g2_power1 = kzg_scheme.public_parameter_group_2[1].clone();

        // Check parameters for G1
        for i in 0..param_size - 1 {
            let elem_first_group_1 = kzg_scheme.public_parameter_group_1[i].clone();
            let elem_next_group_1 = kzg_scheme.public_parameter_group_1[i + 1].clone();
            let elem_next_group_1_target = P::pairing(&elem_next_group_1, &P::G2::generator());
            let elem_next_group_1_target_recomputed = P::pairing(&elem_first_group_1, &g2_power1);
            assert_eq!(
                elem_next_group_1_target_recomputed,
                elem_next_group_1_target
            );
        }

        // Check parameters for G2
        let elem_first_group_2 = kzg_scheme.public_parameter_group_2[0].clone();
        let elem_second_group_2 = kzg_scheme.public_parameter_group_2[1].clone();
        let elem_next_group_2_target = P::pairing(&P::G1::generator(), &elem_second_group_2);
        let elem_next_group_2_target_recomputed = P::pairing(&g1_power1, &elem_first_group_2);

        assert_eq!(
            elem_next_group_2_target_recomputed,
            elem_next_group_2_target
        );
    }

    // Check the size of the KZG being generated.
    fn generation_of_crs<P: Pairing>() {
        let n = 1 << 5;
        let mut prng = test_rng();
        let kzg_scheme = KZGCommitmentScheme::<P>::new(n, &mut prng);
        assert_eq!(kzg_scheme.public_parameter_group_1.len(), n + 1);
        assert_eq!(kzg_scheme.public_parameter_group_2.len(), 2);
    }

    #[test]
    fn test_homomorphic_poly_com_elem() {
        let mut prng = test_rng();
        let pcs = KZGCommitmentSchemeBN254::new(20, &mut prng);
        type Field = Fr;
        let one = Field::one();
        let two = one.add(&one);
        let three = two.add(&one);
        let four = three.add(&one);
        let six = three.add(&three);
        let eight = six.add(&two);
        let poly1 = FpPolynomial::from_coefs(vec![two, three, six]);

        let commitment1 = pcs.commit(&poly1).unwrap();

        let poly2 = FpPolynomial::from_coefs(vec![one, eight, four]);

        let commitment2 = pcs.commit(&poly2).unwrap();

        // Add two polynomials
        let poly_sum = poly1.add(&poly2);
        let commitment_sum = pcs.commit(&poly_sum).unwrap();
        let commitment_sum_computed = commitment1.add(&commitment2);
        assert_eq!(commitment_sum, commitment_sum_computed);

        // Multiplying all the coefficients of a polynomial by some value
        let exponent = four.add(&one);
        let poly1_mult_5 = poly1.mul_scalar(&exponent);
        let commitment_poly1_mult_5 = pcs.commit(&poly1_mult_5).unwrap();
        let commitment_poly1_mult_5_hom = commitment1.mul(&exponent);
        assert_eq!(commitment_poly1_mult_5, commitment_poly1_mult_5_hom);
    }

    #[test]
    fn test_public_parameters() {
        check_public_parameters_generation::<Bn254>();
    }

    #[test]
    fn test_generation_of_crs() {
        generation_of_crs::<Bn254>();
    }

    #[test]
    fn test_commit() {
        let mut prng = test_rng();
        let pcs = KZGCommitmentSchemeBN254::new(10, &mut prng);
        type Field = Fr;
        let one = Field::one();
        let two = one.add(&one);
        let three = two.add(&one);
        let six = three.add(&three);

        let fq_poly = FpPolynomial::from_coefs(vec![two, three, six]);
        let commitment = pcs.commit(&fq_poly).unwrap();

        let coefs_poly_scalar: Vec<_> = fq_poly.get_coefs_ref().iter().cloned().collect();
        let mut expected_committed_value = G1Projective::zero();

        // Doing the multiexp by hand
        for (i, coef) in coefs_poly_scalar.iter().enumerate() {
            let g_i = pcs.public_parameter_group_1[i].clone();
            expected_committed_value = expected_committed_value.add(&g_i.mul(coef));
        }
        assert_eq!(expected_committed_value, commitment.0);
    }

    #[test]
    fn test_eval() {
        let mut prng = test_rng();
        let pcs = KZGCommitmentSchemeBN254::new(10, &mut prng);
        type Field = Fr;
        let one = Field::one();
        let two = one.add(&one);
        let three = two.add(&one);
        let four = three.add(&one);
        let six = three.add(&three);
        let seven = six.add(&one);
        let fq_poly = FpPolynomial::from_coefs(vec![one, two, four]);
        let point = one;
        let max_degree = fq_poly.degree();

        let degree = fq_poly.degree();
        let commitment_value = pcs.commit(&fq_poly).unwrap();

        // Check that an error is returned if the degree of the polynomial exceeds the maximum degree.
        let wrong_max_degree = 1;
        let res = pcs.prove(&fq_poly, &point, wrong_max_degree);
        assert!(res.is_err());

        let proof = pcs.prove(&fq_poly, &point, max_degree).unwrap();

        pcs.verify(&commitment_value, degree, &point, &seven, &proof)
            .unwrap();

        let new_pcs = pcs.shrink_to_verifier_only().unwrap();
        new_pcs
            .verify(&commitment_value, degree, &point, &seven, &proof)
            .unwrap();

        let wrong_eval = one;
        let res = pcs.verify(&commitment_value, degree, &point, &wrong_eval, &proof);
        assert!(res.is_err());
    }
}
