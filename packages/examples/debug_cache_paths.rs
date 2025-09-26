use progresshub_config::environment::get_hf_hub_cache;

fn main() {
    println!("Environment variables:");
    println!("HF_HOME: {:?}", std::env::var("HF_HOME"));
    println!("HF_HUB_CACHE: {:?}", std::env::var("HF_HUB_CACHE"));  
    println!("HUGGINGFACE_HUB_CACHE: {:?}", std::env::var("HUGGINGFACE_HUB_CACHE"));
    
    println!("\nFunction results:");
    let hub_cache = get_hf_hub_cache();
    
    println!("get_hf_hub_cache() returns: {:?}", hub_cache);
    
    println!("\nModel directory construction for 'gpt2':");
    let model_id = "gpt2";
    let model_dir = model_id.replace('/', "--");
    let model_cache_name = format!("models--{model_dir}");
    
    let hub_model_dir = hub_cache.join(&model_cache_name);
    
    println!("Hub model dir: {:?}", hub_model_dir);
    
    println!("\nDirectory existence:");
    println!("Hub model dir exists: {}", hub_model_dir.exists());

    if hub_model_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&hub_model_dir) {
            let count = entries.count();
            println!("Hub model dir has {} entries", count);
        }
    }

    // Check the actual path where I know the model exists
    let actual_path = std::path::PathBuf::from("/Volumes/samsung_t9/ai/models/hub/models--gpt2");
    println!("\nActual known path: {:?}", actual_path);
    println!("Actual path exists: {}", actual_path.exists());
    if actual_path.exists() {
        if let Ok(entries) = std::fs::read_dir(&actual_path) {
            let count = entries.count();
            println!("Actual path has {} entries", count);
        }
    }
}