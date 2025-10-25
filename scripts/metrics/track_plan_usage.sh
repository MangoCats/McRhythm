#!/bin/bash
# Plan Usage Tracking Script for Context Engineering Metrics (PLAN003)
# Usage: ./track_plan_usage.sh

OUTPUT_FILE="project_management/metrics/plan_usage_snapshot_$(date +%Y%m%d).txt"
CSV_FILE="project_management/metrics/plan_usage_log.csv"

# Create metrics directory if it doesn't exist
mkdir -p project_management/metrics

# Generate text report
echo "Plan Usage Tracking Snapshot" > $OUTPUT_FILE
echo "Generated: $(date)" >> $OUTPUT_FILE
echo "---" >> $OUTPUT_FILE
echo "" >> $OUTPUT_FILE

# Count total PLAN folders
TOTAL_PLANS=$(ls -d wip/PLAN*/ 2>/dev/null | wc -l)
echo "Total PLAN### folders in wip/: $TOTAL_PLANS" >> $OUTPUT_FILE
ls -d wip/PLAN*/ 2>/dev/null >> $OUTPUT_FILE

echo "" >> $OUTPUT_FILE
echo "---" >> $OUTPUT_FILE
echo "Specification Issues Detected (All Plans):" >> $OUTPUT_FILE
echo "" >> $OUTPUT_FILE

# Count issues by severity
CRITICAL_COUNT=$(grep -r "PRIORITY: CRITICAL" wip/PLAN*/01_specification_issues.md 2>/dev/null | wc -l)
HIGH_COUNT=$(grep -r "PRIORITY: HIGH" wip/PLAN*/01_specification_issues.md 2>/dev/null | wc -l)
MEDIUM_COUNT=$(grep -r "PRIORITY: MEDIUM" wip/PLAN*/01_specification_issues.md 2>/dev/null | wc -l)
LOW_COUNT=$(grep -r "PRIORITY: LOW" wip/PLAN*/01_specification_issues.md 2>/dev/null | wc -l)

echo "CRITICAL: $CRITICAL_COUNT" >> $OUTPUT_FILE
echo "HIGH: $HIGH_COUNT" >> $OUTPUT_FILE
echo "MEDIUM: $MEDIUM_COUNT" >> $OUTPUT_FILE
echo "LOW: $LOW_COUNT" >> $OUTPUT_FILE

echo "" >> $OUTPUT_FILE
echo "---" >> $OUTPUT_FILE
echo "Critical Issues Detail:" >> $OUTPUT_FILE
grep -r "PRIORITY: CRITICAL" wip/PLAN*/01_specification_issues.md 2>/dev/null >> $OUTPUT_FILE

echo "" >> $OUTPUT_FILE
echo "---" >> $OUTPUT_FILE
echo "High Issues Detail:" >> $OUTPUT_FILE
grep -r "PRIORITY: HIGH" wip/PLAN*/01_specification_issues.md 2>/dev/null >> $OUTPUT_FILE

echo "" >> $OUTPUT_FILE
echo "---" >> $OUTPUT_FILE
echo "Plans Created/Modified in Last 7 Days:" >> $OUTPUT_FILE
find wip/ -maxdepth 1 -type d -name "PLAN*" -mtime -7 >> $OUTPUT_FILE

# Append summary to CSV log
if [ ! -f "$CSV_FILE" ]; then
  echo "Date,Total_Plans,Critical_Issues,High_Issues,Medium_Issues,Low_Issues,New_Plans_Last_7d" > $CSV_FILE
fi

NEW_PLANS_7D=$(find wip/ -maxdepth 1 -type d -name "PLAN*" -mtime -7 | wc -l)
echo "$(date +%Y-%m-%d),$TOTAL_PLANS,$CRITICAL_COUNT,$HIGH_COUNT,$MEDIUM_COUNT,$LOW_COUNT,$NEW_PLANS_7D" >> $CSV_FILE

# Print summary to console
echo "=== Plan Usage Tracking Complete ==="
echo "Total Plans: $TOTAL_PLANS"
echo "Issues Found: C:$CRITICAL_COUNT H:$HIGH_COUNT M:$MEDIUM_COUNT L:$LOW_COUNT"
echo "New Plans (7d): $NEW_PLANS_7D"
echo "Report: $OUTPUT_FILE"
echo "CSV Log: $CSV_FILE"
