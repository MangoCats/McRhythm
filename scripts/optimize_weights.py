#!/usr/bin/env python3
"""
Offline parameter optimizer for single-song identification.

Tests different weight/threshold combinations against collected cross-validation results
to find optimal parameter settings without re-running the full evaluation.
"""

import json
import sys
from dataclasses import dataclass
from typing import List, Dict, Tuple, Optional
from pathlib import Path
import itertools

@dataclass
class WeightConfig:
    """Configuration for weight-based decision making."""
    # Thresholds for rejection
    double_mismatch_threshold: float = 0.70
    title_mismatch_threshold: float = 0.50  # For same-artist wrong-track detection
    artist_mismatch_threshold: float = 0.70  # For cover detection

    # Confidence thresholds
    high_confidence_threshold: float = 0.80
    very_high_confidence_threshold: float = 0.95

    # Weights for combined scoring
    weight_acoustid: float = 0.50
    weight_artist: float = 0.25
    weight_title: float = 0.25

    # Decision rules (enable/disable)
    enable_double_mismatch_rejection: bool = True
    enable_same_artist_wrong_track: bool = False
    enable_cover_detection: bool = False

    def name(self) -> str:
        """Short name for this configuration."""
        parts = []
        parts.append(f"dm{self.double_mismatch_threshold:.2f}")
        if self.enable_same_artist_wrong_track:
            parts.append(f"sawt{self.title_mismatch_threshold:.2f}")
        if self.enable_cover_detection:
            parts.append(f"cov{self.artist_mismatch_threshold:.2f}")
        parts.append(f"w{self.weight_acoustid:.2f}-{self.weight_artist:.2f}-{self.weight_title:.2f}")
        return "_".join(parts)


@dataclass
class EvaluationResult:
    """Result of evaluating a configuration against the data."""
    config: WeightConfig
    total_files: int
    acoustid_found: int
    accepted: int
    rejected: int
    agreement_rate: float

    # Disagreement breakdown
    double_mismatch_rejected: int
    same_artist_wrong_track_rejected: int
    cover_rejected: int

    # Accuracy estimates (assuming ID3 is ground truth for disagreements)
    likely_correct_rejections: int  # Rejected AND double-mismatch
    likely_false_rejections: int    # Rejected BUT not double-mismatch


def apply_config(result: dict, config: WeightConfig) -> Tuple[bool, str]:
    """
    Apply a configuration to a single result and determine accept/reject.

    Returns: (accepted: bool, reason: str)
    """
    artist_sim = result.get('artist_similarity', 0)
    title_sim = result.get('title_similarity', 0)
    acoustid_conf = result.get('acoustid_confidence', 0)

    # No AcoustID match = nothing to decide
    if not result.get('acoustid_mbid'):
        return False, "no_acoustid"

    # Rule 1: Double-mismatch rejection
    if config.enable_double_mismatch_rejection:
        if (artist_sim < config.double_mismatch_threshold and
            title_sim < config.double_mismatch_threshold):
            return False, "double_mismatch"

    # Rule 2: Same-artist wrong-track rejection
    if config.enable_same_artist_wrong_track:
        if (artist_sim >= 0.80 and  # Artist clearly matches
            title_sim < config.title_mismatch_threshold):  # Title clearly doesn't
            return False, "same_artist_wrong_track"

    # Rule 3: Cover detection (reject if title matches but artist doesn't)
    if config.enable_cover_detection:
        if (title_sim >= 0.85 and  # Title matches well
            artist_sim < config.artist_mismatch_threshold):  # Artist doesn't
            return False, "cover_rejected"

    # Rule 4: High confidence acceptance
    if acoustid_conf >= config.high_confidence_threshold:
        return True, "accepted"

    return False, "low_confidence"


def evaluate_config(results: List[dict], config: WeightConfig) -> EvaluationResult:
    """Evaluate a configuration against all results."""
    total = len(results)
    acoustid_found = sum(1 for r in results if r.get('acoustid_mbid'))

    accepted = 0
    rejected = 0
    double_mismatch_rejected = 0
    sawt_rejected = 0
    cover_rejected = 0

    # Track agreement
    sources_agree_accepted = 0
    sources_disagree_accepted = 0

    for r in results:
        if not r.get('acoustid_mbid'):
            continue

        accept, reason = apply_config(r, config)

        if accept:
            accepted += 1
            if r.get('sources_agree'):
                sources_agree_accepted += 1
            else:
                sources_disagree_accepted += 1
        else:
            rejected += 1
            if reason == "double_mismatch":
                double_mismatch_rejected += 1
            elif reason == "same_artist_wrong_track":
                sawt_rejected += 1
            elif reason == "cover_rejected":
                cover_rejected += 1

    # Agreement rate (among accepted)
    agreement_rate = sources_agree_accepted / accepted if accepted > 0 else 0

    # Estimate correctness of rejections
    # Assuming: disagreements are more likely errors, so rejecting disagreements is good
    likely_correct_rejections = double_mismatch_rejected + sawt_rejected + cover_rejected
    likely_false_rejections = rejected - likely_correct_rejections

    return EvaluationResult(
        config=config,
        total_files=total,
        acoustid_found=acoustid_found,
        accepted=accepted,
        rejected=rejected,
        agreement_rate=agreement_rate,
        double_mismatch_rejected=double_mismatch_rejected,
        same_artist_wrong_track_rejected=sawt_rejected,
        cover_rejected=cover_rejected,
        likely_correct_rejections=likely_correct_rejections,
        likely_false_rejections=likely_false_rejections,
    )


def generate_configs() -> List[WeightConfig]:
    """Generate a grid of configurations to test."""
    configs = []

    # Baseline (current implementation)
    configs.append(WeightConfig(
        double_mismatch_threshold=0.70,
        enable_double_mismatch_rejection=True,
        enable_same_artist_wrong_track=False,
        enable_cover_detection=False,
    ))

    # Test different double-mismatch thresholds
    for dm_thresh in [0.60, 0.65, 0.70, 0.75, 0.80]:
        configs.append(WeightConfig(
            double_mismatch_threshold=dm_thresh,
            enable_double_mismatch_rejection=True,
            enable_same_artist_wrong_track=False,
            enable_cover_detection=False,
        ))

    # Fine-grained SAWT thresholds (main focus based on initial results)
    for title_thresh in [0.50, 0.55, 0.60, 0.62, 0.65, 0.70]:
        configs.append(WeightConfig(
            double_mismatch_threshold=0.70,
            title_mismatch_threshold=title_thresh,
            enable_double_mismatch_rejection=True,
            enable_same_artist_wrong_track=True,
            enable_cover_detection=False,
        ))

    # Combined: double-mismatch + same-artist detection with various thresholds
    for dm_thresh, title_thresh in itertools.product([0.65, 0.70, 0.75], [0.55, 0.60, 0.65]):
        configs.append(WeightConfig(
            double_mismatch_threshold=dm_thresh,
            title_mismatch_threshold=title_thresh,
            enable_double_mismatch_rejection=True,
            enable_same_artist_wrong_track=True,
            enable_cover_detection=False,
        ))

    # Test cover detection with SAWT
    for artist_thresh in [0.50, 0.60, 0.70]:
        configs.append(WeightConfig(
            double_mismatch_threshold=0.70,
            artist_mismatch_threshold=artist_thresh,
            enable_double_mismatch_rejection=True,
            enable_same_artist_wrong_track=True,
            title_mismatch_threshold=0.60,  # Best SAWT threshold
            enable_cover_detection=True,
        ))

    return configs


def run_optimization(results_path: str, output_path: Optional[str] = None):
    """Run optimization against results file."""
    with open(results_path, 'r', encoding='utf-8') as f:
        results = json.load(f)

    configs = generate_configs()
    evaluations = []

    print("=" * 90)
    print("PARAMETER OPTIMIZATION RESULTS")
    print("=" * 90)
    print(f"Testing {len(configs)} configurations against {len(results)} files\n")

    for config in configs:
        eval_result = evaluate_config(results, config)
        evaluations.append(eval_result)

    # Sort by agreement rate (higher is better)
    evaluations.sort(key=lambda e: e.agreement_rate, reverse=True)

    # Print results table
    print(f"{'Config':<45} {'Accept':>7} {'Reject':>7} {'Agree%':>8} {'DM':>5} {'SAWT':>5} {'Cov':>5}")
    print("-" * 95)

    for e in evaluations[:25]:
        print(f"{e.config.name():<45} {e.accepted:>7} {e.rejected:>7} "
              f"{e.agreement_rate*100:>7.1f}% {e.double_mismatch_rejected:>5} {e.same_artist_wrong_track_rejected:>5} {e.cover_rejected:>5}")

    # Find best by different criteria
    print("\n" + "=" * 90)
    print("BEST CONFIGURATIONS BY CRITERIA")
    print("=" * 90)

    # Best agreement rate
    best_agree = max(evaluations, key=lambda e: e.agreement_rate)
    print(f"\nBest Agreement Rate: {best_agree.agreement_rate*100:.1f}%")
    print(f"  Config: {best_agree.config.name()}")
    print(f"  Accepted: {best_agree.accepted}, Rejected: {best_agree.rejected}")

    # Best balance (high agreement, low rejection)
    def balance_score(e):
        # Want high agreement AND high acceptance
        return e.agreement_rate * (e.accepted / e.acoustid_found if e.acoustid_found > 0 else 0)

    best_balance = max(evaluations, key=balance_score)
    print(f"\nBest Balance (agreement * acceptance rate):")
    print(f"  Config: {best_balance.config.name()}")
    print(f"  Agreement: {best_balance.agreement_rate*100:.1f}%, Acceptance: {best_balance.accepted/best_balance.acoustid_found*100:.1f}%")

    # Most rejections (aggressive filtering)
    most_reject = max(evaluations, key=lambda e: e.rejected)
    print(f"\nMost Aggressive (highest rejection):")
    print(f"  Config: {most_reject.config.name()}")
    print(f"  Rejected: {most_reject.rejected}, Agreement: {most_reject.agreement_rate*100:.1f}%")

    # Save detailed results
    if output_path:
        output = {
            'source_file': results_path,
            'total_files': len(results),
            'configurations_tested': len(configs),
            'results': [
                {
                    'config': {
                        'double_mismatch_threshold': e.config.double_mismatch_threshold,
                        'title_mismatch_threshold': e.config.title_mismatch_threshold,
                        'enable_double_mismatch': e.config.enable_double_mismatch_rejection,
                        'enable_sawt': e.config.enable_same_artist_wrong_track,
                        'enable_cover': e.config.enable_cover_detection,
                    },
                    'accepted': e.accepted,
                    'rejected': e.rejected,
                    'agreement_rate': e.agreement_rate,
                    'dm_rejected': e.double_mismatch_rejected,
                    'sawt_rejected': e.same_artist_wrong_track_rejected,
                }
                for e in evaluations
            ]
        }
        with open(output_path, 'w', encoding='utf-8') as f:
            json.dump(output, f, indent=2)
        print(f"\nDetailed results saved to: {output_path}")

    return evaluations


if __name__ == '__main__':
    results_path = sys.argv[1] if len(sys.argv) > 1 else r"C:\Users\Mango Cat\Music\single_song_crossval_results.json"
    output_path = sys.argv[2] if len(sys.argv) > 2 else None
    run_optimization(results_path, output_path)
