//! SIMD Genetic Forge
//!
//! This module implements a Genetic Algorithm (GA) to optimize cognitive
//! parameters (e.g., DspConfig). It is designed to be SIMD-friendly for
//! high-throughput fitness evaluation over large trajectory datasets.

use crate::predictor::{DspConfig, DspPredictor};
use rand::prelude::*;

/// A chromosome representing a potential DspConfig.
#[derive(Debug, Clone, Copy)]
pub struct ConfigChromosome {
    pub tau: f32,
    pub beta: i32,
}

impl From<ConfigChromosome> for DspConfig {
    fn from(c: ConfigChromosome) -> Self {
        DspConfig {
            tau: c.tau,
            beta: c.beta,
            max_speculative_steps: 10,
        }
    }
}

/// The Genetic Forge engine.
pub struct GeneticForge {
    pub population_size: usize,
    pub mutation_rate: f32,
}

impl GeneticForge {
    pub fn new(population_size: usize, mutation_rate: f32) -> Self {
        Self {
            population_size,
            mutation_rate,
        }
    }

    /// Evolves the population to find the optimal DspConfig.
    /// 
    /// # Arguments
    /// * `training_data` - Pairs of (complexity, actual_optimal_k)
    pub fn evolve(&self, training_data: &[(f32, u32)]) -> DspConfig {
        let mut rng = thread_rng();
        let mut population: Vec<ConfigChromosome> = (0..self.population_size)
            .map(|_| ConfigChromosome {
                tau: rng.gen_range(0.1..0.9),
                beta: rng.gen_range(-2..3),
            })
            .collect();

        // Perform 10 generations of evolution
        for _ in 0..10 {
            // 1. Evaluate Fitness (structured for potential SIMD autovectorization)
            let mut fitness_scores: Vec<(f32, ConfigChromosome)> = population
                .iter()
                .map(|&c| {
                    let score = self.calculate_fitness(c, training_data);
                    (score, c)
                })
                .collect();

            // 2. Selection (Sort by fitness descending: higher is better)
            fitness_scores.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());

            // 3. Breeding (Top 50% survive and reproduce)
            let survivors: Vec<ConfigChromosome> = fitness_scores
                .iter()
                .take(self.population_size / 2)
                .map(|(_, c)| *c)
                .collect();

            let mut next_gen = survivors.clone();
            while next_gen.len() < self.population_size {
                let parent1 = survivors.choose(&mut rng).unwrap();
                let parent2 = survivors.choose(&mut rng).unwrap();
                
                // Crossover & Mutation
                let mut child = ConfigChromosome {
                    tau: if rng.gen() { parent1.tau } else { parent2.tau },
                    beta: if rng.gen() { parent1.beta } else { parent2.beta },
                };

                if rng.gen::<f32>() < self.mutation_rate {
                    child.tau += rng.gen_range(-0.1..0.1);
                    child.tau = child.tau.clamp(0.1, 0.9);
                }

                next_gen.push(child);
            }
            population = next_gen;
        }

        population[0].into()
    }

    /// Calculates fitness based on inverse loss across training data.
    fn calculate_fitness(&self, chromosome: ConfigChromosome, training_data: &[(f32, u32)]) -> f32 {
        let mut predictor = DspPredictor::new(chromosome.into()).unwrap();
        let mut total_loss = 0.0;

        for &(complexity, actual) in training_data {
            let predicted = predictor.predict_optimal_k(complexity);
            // fitness = 1 / (1 + total_loss)
            let diff = actual as f32 - predicted as f32;
            total_loss += diff.powi(2);
        }

        1.0 / (1.0 + total_loss)
    }
}
