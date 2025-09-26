//! Immutable state management for progress tracking
//!
//! This module provides pure functions for creating and updating immutable
//! progress state with structural sharing and zero-mutation guarantees.

use crate::calculator::types::{
    ImmutableFileProgress, ImmutableModelProgress, ImmutableProgressState, RawModelProgress,
};
use std::time::SystemTime;

/// Create new immutable progress state from manifest and model data
///
/// Pure constructor function that builds complete immutable progress state
/// from raw input data (manifest files, model lists, etc.)
///
/// # Arguments
/// * `manifest_data` - Manifest metadata as key-value pairs
/// * `model_data` - Model data as (model_id, files: (name, downloaded, total, cached))
///
/// # Returns
/// New ImmutableProgressState with complete progress representation
pub fn from_manifest_and_progress(
    manifest_data: &std::collections::HashMap<String, String>,
    model_data: &[RawModelProgress],
) -> ImmutableProgressState {
    let manifest_map: im::OrdMap<String, String> = manifest_data
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();

    let models: im::Vector<ImmutableModelProgress> = model_data
        .iter()
        .map(|(model_id, files)| {
            let file_progresses: im::Vector<ImmutableFileProgress> = files
                .iter()
                .map(|(name, downloaded, total, cached)| {
                    let mut file_metadata = im::OrdMap::new();
                    file_metadata.insert("name".to_string(), name.clone());

                    ImmutableFileProgress {
                        file_id: file_metadata,
                        file_name: name.clone(),
                        bytes_downloaded: *downloaded,
                        total_bytes: *total,
                        is_cached: *cached,
                        chunks_downloaded: im::Vector::new(),
                    }
                })
                .collect();

            let (model_downloaded, model_total) = files.iter().fold(
                (0u64, 0u64),
                |(down, total), (_, downloaded, file_total, _)| {
                    (down + downloaded, total + file_total)
                },
            );

            ImmutableModelProgress {
                model_id: model_id.clone(),
                bytes_downloaded: model_downloaded,
                total_bytes: model_total,
                files: file_progresses,
                is_cached: files.iter().all(|(_, _, _, cached)| *cached),
                metadata: im::OrdMap::new(),
            }
        })
        .collect();

    let (overall_downloaded, overall_total) =
        models.iter().fold((0u64, 0u64), |(down, total), model| {
            (down + model.bytes_downloaded, total + model.total_bytes)
        });

    ImmutableProgressState {
        models,
        overall_bytes_downloaded: overall_downloaded,
        overall_total_bytes: overall_total,
        manifest_data: manifest_map,
        timestamp: SystemTime::now(),
    }
}

/// Update immutable state with new file progress
///
/// Pure function returning new immutable state with the file update
/// incorporated, without mutating any existing data. Creates new models
/// automatically if they don't exist.
///
/// # Arguments
/// * `state` - Current immutable progress state
/// * `model_id` - ID of the model to update
/// * `file_update` - New immutable file progress information
///
/// # Returns
/// New ImmutableProgressState with the file progress incorporated
pub fn with_updated_file(
    state: ImmutableProgressState,
    model_id: &str,
    file_update: ImmutableFileProgress,
) -> ImmutableProgressState {
    // Check if model exists
    let model_exists = state.models.iter().any(|model| model.model_id == model_id);

    let updated_models: im::Vector<ImmutableModelProgress> = if model_exists {
        // Update existing model
        state
            .models
            .iter()
            .map(|model| {
                if model.model_id == model_id {
                    let updated_files = if let Some(existing_idx) = model
                        .files
                        .iter()
                        .position(|f| f.file_id.get("name") == file_update.file_id.get("name"))
                    {
                        model.files.update(existing_idx, file_update.clone())
                    } else {
                        let mut new_files = model.files.clone();
                        new_files.push_back(file_update.clone());
                        new_files
                    };

                    let (total_downloaded, total_bytes) =
                        updated_files
                            .iter()
                            .fold((0u64, 0u64), |(down, total), file| {
                                (down + file.bytes_downloaded, total + file.total_bytes)
                            });

                    ImmutableModelProgress {
                        model_id: model.model_id.clone(),
                        bytes_downloaded: total_downloaded,
                        total_bytes,
                        files: updated_files.clone(),
                        is_cached: updated_files.iter().all(|file| file.is_cached),
                        metadata: model.metadata.clone(),
                    }
                } else {
                    model.clone()
                }
            })
            .collect()
    } else {
        // Create new model with this file
        let mut new_files = im::Vector::new();
        new_files.push_back(file_update.clone());

        let new_model = ImmutableModelProgress {
            model_id: model_id.to_string(),
            bytes_downloaded: file_update.bytes_downloaded,
            total_bytes: file_update.total_bytes,
            files: new_files,
            is_cached: file_update.is_cached,
            metadata: im::OrdMap::new(),
        };

        let mut updated_models = state.models.clone();
        updated_models.push_back(new_model);
        updated_models
    };

    let (overall_downloaded, overall_total) = updated_models
        .iter()
        .fold((0u64, 0u64), |(down, total), model| {
            (down + model.bytes_downloaded, total + model.total_bytes)
        });

    ImmutableProgressState {
        models: updated_models,
        overall_bytes_downloaded: overall_downloaded,
        overall_total_bytes: overall_total,
        manifest_data: state.manifest_data,
        timestamp: SystemTime::now(),
    }
}
