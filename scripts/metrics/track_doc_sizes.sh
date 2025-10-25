#!/bin/bash
# Document Size Tracking Script for Context Engineering Metrics (PLAN003)
# Usage: ./track_doc_sizes.sh <week_number>
# Example: ./track_doc_sizes.sh 1

WEEK_NUM=$1

if [ -z "$WEEK_NUM" ]; then
  echo "Error: Week number required"
  echo "Usage: ./track_doc_sizes.sh <week_number>"
  exit 1
fi

OUTPUT_FILE="project_management/metrics/doc_sizes_week_${WEEK_NUM}.txt"
CSV_FILE="project_management/metrics/document_size_log.csv"

# Create metrics directory if it doesn't exist
mkdir -p project_management/metrics

# Generate text report
echo "Document Size Tracking - Week ${WEEK_NUM}" > $OUTPUT_FILE
echo "Generated: $(date)" >> $OUTPUT_FILE
echo "Period: Last 7 days from $(date -d '7 days ago' +%Y-%m-%d) to $(date +%Y-%m-%d)" >> $OUTPUT_FILE
echo "---" >> $OUTPUT_FILE
echo "" >> $OUTPUT_FILE

echo "All .md files created/modified in last 7 days:" >> $OUTPUT_FILE
find docs/ wip/ workflows/ -name "*.md" -mtime -7 -exec wc -l {} \; | sort -n -r >> $OUTPUT_FILE

echo "" >> $OUTPUT_FILE
echo "---" >> $OUTPUT_FILE
echo "Summary files only (modular documents):" >> $OUTPUT_FILE
find docs/ wip/ -name "00_SUMMARY.md" -mtime -7 -exec wc -l {} \; >> $OUTPUT_FILE

echo "" >> $OUTPUT_FILE
echo "---" >> $OUTPUT_FILE
echo "Full documents (modular - for archival comparison):" >> $OUTPUT_FILE
find docs/ wip/ -name "FULL_DOCUMENT.md" -mtime -7 -exec wc -l {} \; >> $OUTPUT_FILE

# Append to CSV log (if file exists, otherwise create with header)
if [ ! -f "$CSV_FILE" ]; then
  echo "Week,Date,Document,Lines,Type,Notes" > $CSV_FILE
fi

# Extract data and append to CSV
find docs/ wip/ workflows/ -name "*.md" -mtime -7 | while read file; do
  lines=$(wc -l < "$file")
  type="single"

  if [[ "$file" == *"00_SUMMARY.md" ]]; then
    type="modular_summary"
  elif [[ "$file" == *"FULL_DOCUMENT.md" ]]; then
    type="modular_full"
  fi

  echo "$WEEK_NUM,$(date +%Y-%m-%d),$file,$lines,$type," >> $CSV_FILE
done

echo "" >> $OUTPUT_FILE
echo "Data appended to: $CSV_FILE"
echo "Report generated: $OUTPUT_FILE"

# Print summary to console
echo "=== Week ${WEEK_NUM} Document Size Tracking Complete ==="
echo "Total documents found: $(find docs/ wip/ workflows/ -name "*.md" -mtime -7 | wc -l)"
echo "Report: $OUTPUT_FILE"
echo "CSV Log: $CSV_FILE"
