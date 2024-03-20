use std::ops::{Add, Mul};
use k256::{FieldBytes, ProjectivePoint, Scalar};
use k256::elliptic_curve::group::GroupEncoding;
use k256::elliptic_curve::PrimeField;
use merlin::Transcript;
use crate::util::*;

pub struct WeightNormLinearArgument {
    pub g: ProjectivePoint,
    pub g_vec: Vec<ProjectivePoint>,
    pub h_vec: Vec<ProjectivePoint>,
    pub c: Vec<Scalar>,
    pub rho: Scalar,
    pub mu: Scalar,
}

#[derive(Debug)]
pub struct Proof {
    pub r: Vec<ProjectivePoint>,
    pub x: Vec<ProjectivePoint>,
    pub l: Vec<Scalar>,
    pub n: Vec<Scalar>,
}

impl WeightNormLinearArgument {
    pub fn commit(&self, l: &Vec<Scalar>, n: &Vec<Scalar>) -> ProjectivePoint {
        let v = vector_mul(&self.c, l).add(weight_vector_mul(n, n, &self.mu));
        self.
            g.mul(v).
            add(vector_mul(&self.h_vec, l)).
            add(vector_mul(&self.g_vec, n))
    }

    pub fn verify(&self, commitment: &ProjectivePoint, t: &mut Transcript, proof: Proof) -> bool {
        if proof.x.len() != proof.r.len() {
            return false;
        }

        if proof.x.len() == 0 {
            return commitment.eq(&self.commit(&proof.l, &proof.n));
        }

        let (c0, c1) = reduce(&self.c);
        let (g0, g1) = reduce(&self.g_vec);
        let (h0, h1) = reduce(&self.h_vec);

        t.append_message(b"com", commitment.to_bytes().as_slice());
        t.append_message(b"x", proof.x.last().unwrap().to_bytes().as_slice());
        t.append_message(b"r", proof.r.last().unwrap().to_bytes().as_slice());
        t.append_u64(b"l.sz", self.h_vec.len() as u64);
        t.append_u64(b"n.sz", self.g_vec.len() as u64);

        let mut buf = [0u8; 32];
        t.challenge_bytes(b"y", &mut buf);
        let y = k256::Scalar::from_repr(*FieldBytes::from_slice(&buf)).unwrap();

        let h_ = vector_add(&h0, &vector_mul_on_scalar(&h1, &y));
        let g_ = vector_add(&vector_mul_on_scalar(&g0, &self.rho), &vector_mul_on_scalar(&g1, &y));
        let c_ = vector_add(&c0, &vector_mul_on_scalar(&c1, &y));

        let com_ = commitment.
            add(&proof.x.last().unwrap().mul(&y)).
            add(&proof.r.last().unwrap().mul(&y.mul(&y).sub(&Scalar::ONE)));

        let wnla = WeightNormLinearArgument {
            g: self.g,
            g_vec: g_,
            h_vec: h_,
            c: c_,
            rho: self.mu,
            mu: self.mu.mul(&self.mu),
        };

        let proof_ = Proof {
            r: proof.r[..proof.r.len() - 1].to_vec(),
            x: proof.x[..proof.x.len() - 1].to_vec(),
            l: proof.l,
            n: proof.n,
        };

        return wnla.verify(&com_, t, proof_);
    }

    pub fn prove(&self, commitment: &ProjectivePoint, t: &mut Transcript, l: Vec<Scalar>, n: Vec<Scalar>) -> Proof {

        // TODO remove
        println!("WNLA secret: {:?}", vector_mul(&self.c, &l).add(weight_vector_mul(&n, &n, &self.mu)));

        if l.len() + n.len() < 6 {
            return Proof {
                r: vec![],
                x: vec![],
                l,
                n,
            };
        }

        let rho_inv = self.rho.invert().unwrap();

        let (c0, c1) = reduce(&self.c);
        let (l0, l1) = reduce(&l);
        let (n0, n1) = reduce(&n);
        let (g0, g1) = reduce(&self.g_vec);
        let (h0, h1) = reduce(&self.h_vec);

        let mu2 = self.mu.mul(&self.mu);

        let vx = weight_vector_mul(&n0, &n1, &mu2).
            mul(&rho_inv.mul(&Scalar::from(2u32))).
            add(&vector_mul(&c0, &l1)).
            add(&vector_mul(&c1, &l0));

        let vr = weight_vector_mul(&n1, &n1, &mu2).add(&vector_mul(&c1, &l1));

        let x = self.g.mul(vx).
            add(&vector_mul(&h0, &l1)).
            add(&vector_mul(&h1, &l0)).
            add(&vector_mul(&g0, &vector_mul_on_scalar(&n1, &self.rho))).
            add(&vector_mul(&g1, &vector_mul_on_scalar(&n0, &rho_inv)));

        let r = self.g.mul(vr).
            add(vector_mul(&h1, &l1)).
            add(vector_mul(&g1, &n1));

        t.append_message(b"com", commitment.to_bytes().as_slice());
        t.append_message(b"x", x.to_bytes().as_slice());
        t.append_message(b"r", r.to_bytes().as_slice());
        t.append_u64(b"l.sz", l.len() as u64);
        t.append_u64(b"n.sz", n.len() as u64);

        let mut buf = [0u8; 32];
        t.challenge_bytes(b"y", &mut buf);
        let y = k256::Scalar::from_repr(*FieldBytes::from_slice(&buf)).unwrap();

        let h_ = vector_add(&h0, &vector_mul_on_scalar(&h1, &y));
        let g_ = vector_add(&vector_mul_on_scalar(&g0, &self.rho), &vector_mul_on_scalar(&g1, &y));
        let c_ = vector_add(&c0, &vector_mul_on_scalar(&c1, &y));

        let l_ = vector_add(&l0, &vector_mul_on_scalar(&l1, &y));
        let n_ = vector_add(&vector_mul_on_scalar(&n0, &rho_inv), &vector_mul_on_scalar(&n1, &y));

        let wnla = WeightNormLinearArgument {
            g: self.g,
            g_vec: g_,
            h_vec: h_,
            c: c_,
            rho: self.mu,
            mu: mu2,
        };

        let mut proof = wnla.prove(&wnla.commit(&l_, &n_), t, l_, n_);
        proof.r.push(r);
        proof.x.push(x);
        proof
    }
}
