use anyhow::{Context, Result};
use candle_core::{Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::bert::{BertModel, Config};
use std::fs;
use std::path::{Path, PathBuf};
use tokenizers::Tokenizer;
use tracing::{info, warn};

/// Embedding service using Candle + sentence-transformers/all-MiniLM-L6-v2
pub struct EmbeddingService {
    model: BertModel,
    tokenizer: Tokenizer,
    device: Device,
    dimensions: usize,
}

/// Download a file from Hugging Face Hub with proper redirect handling
fn download_hf_file(model_name: &str, filename: &str, cache_dir: &Path) -> Result<PathBuf> {
    let model_cache = cache_dir.join(model_name.replace('/', "--"));
    fs::create_dir_all(&model_cache)?;

    let output_path = model_cache.join(filename);

    // Skip download if file already exists
    if output_path.exists() {
        info!("Using cached file: {}", output_path.display());
        return Ok(output_path);
    }

    let url = format!(
        "https://huggingface.co/{}/resolve/main/{}",
        model_name, filename
    );

    info!("Downloading {} from {}", filename, url);

    let response = ureq::get(&url)
        .call()
        .with_context(|| format!("Failed to download {}", filename))?;

    let mut file = fs::File::create(&output_path)
        .with_context(|| format!("Failed to create file {}", output_path.display()))?;

    std::io::copy(&mut response.into_reader(), &mut file)
        .with_context(|| format!("Failed to write {}", filename))?;

    info!("Downloaded {} successfully", filename);
    Ok(output_path)
}

impl EmbeddingService {
    /// Create embedding service with a specific model
    pub fn with_model(
        model_name: &str,
        dimensions: usize,
        cache_dir: Option<PathBuf>,
    ) -> Result<Self> {
        info!("Initializing embedding service with model: {}", model_name);
        info!("Downloading model files from Hugging Face Hub...");

        // Set up cache directory
        let cache = cache_dir.unwrap_or_else(|| {
            dirs::cache_dir()
                .unwrap_or_else(|| PathBuf::from(".cache"))
                .join("ltm-mcp")
                .join("models")
        });

        // Download required files using manual download with redirect support
        info!("Downloading tokenizer.json...");
        let tokenizer_path = download_hf_file(model_name, "tokenizer.json", &cache)?;

        info!("Downloading config.json...");
        let config_path = download_hf_file(model_name, "config.json", &cache)?;

        info!("Downloading model weights (model.safetensors or pytorch_model.bin)...");
        let weights_path = download_hf_file(model_name, "model.safetensors", &cache)
            .or_else(|_| {
                warn!("model.safetensors not found, trying pytorch_model.bin");
                download_hf_file(model_name, "pytorch_model.bin", &cache)
            })
            .context("Failed to download model weights")?;

        // Load tokenizer
        info!("Loading tokenizer...");
        let tokenizer = Tokenizer::from_file(tokenizer_path)
            .map_err(|e| anyhow::anyhow!("Failed to load tokenizer: {}", e))?;

        // Load config
        info!("Loading model config...");
        let config_content =
            std::fs::read_to_string(config_path).context("Failed to read config.json")?;
        let config: Config =
            serde_json::from_str(&config_content).context("Failed to parse config.json")?;

        // Load model weights
        info!("Loading model weights...");
        let device = Device::Cpu;
        let vb = if weights_path.extension().and_then(|s| s.to_str()) == Some("safetensors") {
            unsafe {
                VarBuilder::from_mmaped_safetensors(
                    &[weights_path],
                    candle_core::DType::F32,
                    &device,
                )?
            }
        } else {
            VarBuilder::from_pth(&weights_path, candle_core::DType::F32, &device)?
        };

        let model = BertModel::load(vb, &config).context("Failed to load BERT model")?;

        info!("Embedding service initialized successfully!");
        info!("Model: {}, Dimensions: {}", model_name, dimensions);

        Ok(Self {
            model,
            tokenizer,
            device,
            dimensions,
        })
    }

    /// Generate embedding for a single text
    pub fn embed(&self, text: &str) -> Result<Vec<f32>> {
        // Tokenize input
        let encoding = self
            .tokenizer
            .encode(text, true)
            .map_err(|e| anyhow::anyhow!("Tokenization failed: {}", e))?;

        let tokens = encoding.get_ids();
        let token_ids = Tensor::new(tokens, &self.device)?.unsqueeze(0)?; // Add batch dimension

        // Create attention mask
        let attention_mask =
            Tensor::ones((1, tokens.len()), candle_core::DType::U32, &self.device)?;

        // Forward pass (token_type_ids set to None for sentence embeddings)
        let embeddings = self.model.forward(&token_ids, &attention_mask, None)?;

        // Mean pooling (average all token embeddings)
        let pooled = embeddings.mean(1)?; // Mean across sequence dimension

        // Normalize to unit length (for cosine similarity)
        let pooled_vec = pooled.squeeze(0)?.to_vec1::<f32>()?;
        let norm = pooled_vec.iter().map(|x| x * x).sum::<f32>().sqrt();
        let normalized: Vec<f32> = pooled_vec.iter().map(|x| x / norm).collect();

        // Verify dimensions
        if normalized.len() != self.dimensions {
            anyhow::bail!(
                "Embedding dimension mismatch: expected {}, got {}",
                self.dimensions,
                normalized.len()
            );
        }

        Ok(normalized)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore] // Requires model download
    fn test_embedding_generation() {
        let service =
            EmbeddingService::with_model("sentence-transformers/all-MiniLM-L6-v2", 384, None)
                .unwrap();

        let text = "This is a test sentence";
        let embedding = service.embed(text).unwrap();

        assert_eq!(embedding.len(), 384);

        // Check normalization (L2 norm should be ~1.0)
        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 0.01);
    }

    #[test]
    #[ignore] // Requires model download
    fn test_semantic_similarity() {
        let service =
            EmbeddingService::with_model("sentence-transformers/all-MiniLM-L6-v2", 384, None)
                .unwrap();

        let text1 = "The cat sits on the mat";
        let text2 = "A feline rests on a rug";
        let text3 = "Python is a programming language";

        let emb1 = service.embed(text1).unwrap();
        let emb2 = service.embed(text2).unwrap();
        let emb3 = service.embed(text3).unwrap();

        // Cosine similarity (dot product for normalized vectors)
        let sim_12: f32 = emb1.iter().zip(&emb2).map(|(a, b)| a * b).sum();
        let sim_13: f32 = emb1.iter().zip(&emb3).map(|(a, b)| a * b).sum();

        // Similar sentences should have higher similarity
        assert!(sim_12 > sim_13);
        assert!(sim_12 > 0.5); // Reasonable similarity threshold
    }
}
