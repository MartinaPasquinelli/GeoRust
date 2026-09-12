use crate::errors::AuthError;
use rand::RngExt;
use sha2::{Digest, Sha256};

// Genera un hash SHA-256 con salt casuale a partire da una password in chiaro.
pub fn hash_password(password: &str) -> Result<String, AuthError> {
    // Genera un salt casuale di 16 byte
    let mut salt = [0u8; 16];
    rand::rng().fill(&mut salt);

    // Crea l'hasher SHA-256
    let mut hasher = Sha256::new();

    // Aggiunge password e salt all'hash
    hasher.update(password.as_bytes());
    hasher.update(&salt);

    // Calcola l'hash
    let hash = hasher.finalize();

    // Salva salt + hash in formato salt:hash
    Ok(format!("{}:{}", hex::encode(salt), hex::encode(hash)))
}

// Verifica se una password in chiaro corrisponde all'hash SHA-256 salvato nel DB.
pub fn verify_password(password: &str, password_hash: &str) -> Result<bool, AuthError> {
    let parts: Vec<&str> = password_hash.split(':').collect();

    if parts.len() != 2 {
        return Err(AuthError::PasswordHashError(
            "Formato hash non valido".to_string(),
        ));
    }

    let salt =
        hex::decode(parts[0]).map_err(|err| AuthError::PasswordHashError(err.to_string()))?;

    let expected_hash =
        hex::decode(parts[1]).map_err(|err| AuthError::PasswordHashError(err.to_string()))?;

    // Calcola nuovamente SHA-256 usando password + salt
    let mut hasher = Sha256::new();
    hasher.update(password.as_bytes());
    hasher.update(&salt);

    let calculated_hash = hasher.finalize();

    // Confronta l'hash calcolato con quello salvato
    Ok(calculated_hash.as_slice() == expected_hash.as_slice())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_and_verify_success() {
        let pwd = "super_secret_password_123";
        let hash = hash_password(pwd).expect("Hashing fallito");
        assert!(hash.contains(':'));
        let verified = verify_password(pwd, &hash).expect("Verifica fallita");
        assert!(verified);
    }

    #[test]
    fn test_verify_wrong_password() {
        let pwd = "correct_password";
        let hash = hash_password(pwd).expect("Hashing fallito");
        let verified = verify_password("wrong_password", &hash).expect("Verifica fallita");
        assert!(!verified);
    }

    #[test]
    fn test_verify_malformed_hash() {
        let res1 = verify_password("pwd", "not_an_hash");
        assert!(res1.is_err());

        let res2 = verify_password("pwd", "invalidhex:invalidhex");
        assert!(res2.is_err());
    }

    #[test]
    fn test_salt_uniqueness() {
        let pwd = "identical_password";
        let hash1 = hash_password(pwd).unwrap();
        let hash2 = hash_password(pwd).unwrap();
        // Con due salt casuali, gli hash devono essere differenti
        assert_ne!(hash1, hash2);
    }
}
