use sha2::{Digest, Sha256};

trait Hasher {
    fn clone(&self) -> Box<dyn Hasher>;
    fn update(&mut self, data: &[u8]);
    fn finalize(&mut self) -> [u8; 32];
}

struct Sha256Hash(Sha256);

impl Sha256Hash {
    fn new() -> Self {
        Self(Sha256::new())
    }
}

impl Hasher for Sha256Hash {
    fn clone(&self) -> Box<dyn Hasher> {
        Box::new(Self(self.0.clone()))
    }

    fn update(&mut self, data: &[u8]) {
        self.0.update(data);
    }

    fn finalize(&mut self) -> [u8; 32] {
        self.0.clone().finalize().into()
    }
}

struct RecursiveHash {
    inner: Box<dyn Hasher>,
    outer: Box<dyn Hasher>,
    ipad: [u8; 64],
    opad: [u8; 64],
}

impl RecursiveHash {
    fn new(key: &[u8], hash: Box<dyn Hasher>) -> Self {
        let mut ipad = [0u8; 64];
        let mut opad = [0u8; 64];

        ipad[..key.len()].copy_from_slice(&key);
        opad[..key.len()].copy_from_slice(&key);

        for b in ipad.iter_mut() {
            *b ^= 0x36;
        }

        for b in opad.iter_mut() {
            *b ^= 0x5c;
        }

        let mut inner = hash.clone();
        let outer = hash;

        inner.update(&ipad);
        Self {
            inner,
            outer,
            ipad,
            opad,
        }
    }
}

impl Hasher for RecursiveHash {
    fn clone(&self) -> Box<dyn Hasher> {
        let inner = self.inner.clone();
        let outer = self.outer.clone();
        let ipad = self.ipad.clone();
        let opad = self.opad.clone();

        Box::new(Self {
            inner,
            outer,
            ipad,
            opad,
        })
    }

    fn update(&mut self, data: &[u8]) {
        self.inner.update(data);
    }

    fn finalize(&mut self) -> [u8; 32] {
        let result: [u8; 32] = self.inner.finalize().into();
        self.outer.update(&self.opad);
        self.outer.update(&result);
        self.outer.finalize().into()
    }
}

pub fn kdf(key: &[u8], path: &[&[u8]]) -> [u8; 32] {
    let mut current = Box::new(RecursiveHash::new(
        b"VMess AEAD KDF",
        Box::new(Sha256Hash::new()),
    ));

    for p in path.into_iter() {
        current = Box::new(RecursiveHash::new(p, current));
    }

    current.update(key);
    current.finalize()
}

#[cfg(test)]
mod tests {
    use super::*;
    use md5::Md5;

    #[test]
    fn test_kdf() {
        let uuid = uuid::uuid!("96850032-1b92-46e9-a4f2-b99631456894").as_bytes();
        let key = crate::md5!(&uuid, b"c48619fe-8f02-49e0-b9e9-edf763e17e21");

        let res = kdf(&key, &[b"AES Auth ID Encryption"]);

        assert_eq!(
            res[..16],
            [117, 82, 144, 159, 147, 65, 74, 253, 91, 74, 70, 84, 114, 118, 203, 30]
        );
    }
}
