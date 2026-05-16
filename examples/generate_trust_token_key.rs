use flowfull_rust_starter::tokens::generate_keypair;

fn main() {
    let (public_key, private_key) = generate_keypair();
    println!("TRUST_TOKEN_PUBLIC_KEY={public_key}");
    println!("TRUST_TOKEN_PRIVATE_KEY={private_key}");
}
