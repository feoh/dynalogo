#!/usr/bin/env ruby
# frozen_string_literal: true

require "yaml"

workflow_dir = File.join(__dir__, "..", ".github", "workflows")
files = Dir[File.join(workflow_dir, "*.yml")].sort + Dir[File.join(workflow_dir, "*.yaml")].sort

abort "no workflow files found in #{workflow_dir}" if files.empty?

errors = []

files.each do |path|
  begin
    data = YAML.safe_load_file(path, aliases: true)
  rescue Psych::SyntaxError => e
    errors << "#{path}: YAML syntax error: #{e.message}"
    next
  end

  unless data.is_a?(Hash)
    errors << "#{path}: workflow must be a mapping"
    next
  end

  name = data["name"]
  trigger = data["on"] || data[:on] || data[true]
  jobs = data["jobs"]

  errors << "#{path}: missing non-empty name" unless name.is_a?(String) && !name.strip.empty?
  errors << "#{path}: missing on trigger" if trigger.nil?
  unless jobs.is_a?(Hash) && !jobs.empty?
    errors << "#{path}: missing non-empty jobs mapping"
    next
  end

  jobs.each do |job_name, job|
    unless job.is_a?(Hash)
      errors << "#{path}: job #{job_name.inspect} must be a mapping"
      next
    end

    has_runner = job.key?("runs-on") || job.key?("uses")
    errors << "#{path}: job #{job_name.inspect} missing runs-on or uses" unless has_runner

    next if job.key?("uses")

    steps = job["steps"]
    unless steps.is_a?(Array) && !steps.empty?
      errors << "#{path}: job #{job_name.inspect} missing non-empty steps"
      next
    end

    steps.each_with_index do |step, index|
      unless step.is_a?(Hash)
        errors << "#{path}: job #{job_name.inspect} step #{index + 1} must be a mapping"
        next
      end
      next if step.key?("run") || step.key?("uses")

      errors << "#{path}: job #{job_name.inspect} step #{index + 1} missing run or uses"
    end
  end
end

if errors.any?
  warn errors.join("\n")
  exit 1
end

puts "Validated #{files.length} GitHub workflow file(s)"
