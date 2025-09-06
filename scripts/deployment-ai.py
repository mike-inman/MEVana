#!/usr/bin/env python3
"""
DeploymentAI - Learn from deployment failures and suggest fixes
Because humans shouldn't have to remember every edge case! 🤖
"""

import re
import json
import sys
from datetime import datetime
from pathlib import Path
from typing import Dict, List, Any, Optional

class DeploymentDoctor:
    def __init__(self):
        self.knowledge_base = {
            # Port conflicts
            r"network.*already exists": {
                "category": "infrastructure",
                "severity": "medium", 
                "fix": "Check if network exists before creating",
                "code_fix": 'if ! docker network ls | grep -q mevana_net; then\n  docker network create mevana_net\nfi',
                "confidence": 0.95
            },
            
            r"address already in use.*:(\d+)": {
                "category": "port_conflict",
                "severity": "high",
                "fix": "Port {port} is occupied. Kill process or use different port",
                "code_fix": "lsof -ti:{port} | xargs kill -9",
                "confidence": 0.98
            },
            
            r"could not find `protoc`": {
                "category": "dependencies",
                "severity": "high",
                "fix": "Install Protocol Buffers compiler",
                "code_fix": "apt-get install -y protobuf-compiler",
                "confidence": 0.99
            },
            
            r"large.*enum.*variant": {
                "category": "performance",
                "severity": "low",
                "fix": "Use #[allow(clippy::large_enum_variant)] for hotpath code",
                "code_fix": "#[allow(clippy::large_enum_variant)]",
                "confidence": 0.90
            },
            
            r"here-document.*delimited.*EOF": {
                "category": "syntax",
                "severity": "high", 
                "fix": "Malformed heredoc - check for missing EOF or quote mismatches",
                "code_fix": "Replace complex heredocs with individual commands",
                "confidence": 0.92
            },
            
            r"binary operation.*cannot be applied.*PartialEq": {
                "category": "traits",
                "severity": "medium",
                "fix": "Add #[derive(PartialEq)] to enable comparisons",
                "code_fix": "#[derive(PartialEq)]",
                "confidence": 0.96
            },
            
            r"pseudo-terminal.*not.*allocated": {
                "category": "ssh",
                "severity": "low",
                "fix": "Add -T flag to SSH commands in CI/CD",
                "code_fix": "ssh -T $USER@$HOST",
                "confidence": 0.85
            },
            
            r"formatting.*cargo fmt": {
                "category": "formatting", 
                "severity": "medium",
                "fix": "Run 'cargo fmt --all' before committing",
                "code_fix": "cargo fmt --all",
                "confidence": 0.99
            }
        }
        
        self.deployment_history = []
        self.load_history()
    
    def load_history(self):
        """Load past deployment failures to learn from them"""
        history_file = Path("deployment_history.json")
        if history_file.exists():
            with open(history_file) as f:
                self.deployment_history = json.load(f)
    
    def save_history(self):
        """Save deployment attempt for future learning"""
        with open("deployment_history.json", "w") as f:
            json.dump(self.deployment_history, f, indent=2)
    
    def analyze_failure(self, error_log: str) -> List[Dict[str, Any]]:
        """AI-powered failure analysis"""
        suggestions = []
        
        for pattern, info in self.knowledge_base.items():
            matches = re.finditer(pattern, error_log, re.IGNORECASE | re.MULTILINE)
            
            for match in matches:
                suggestion = info.copy()
                suggestion["pattern_matched"] = pattern
                suggestion["error_context"] = self._get_error_context(error_log, match)
                
                # Extract dynamic values (like port numbers)
                if "{port}" in suggestion["fix"]:
                    port = match.group(1) if match.groups() else "UNKNOWN"
                    suggestion["fix"] = suggestion["fix"].format(port=port)
                    suggestion["code_fix"] = suggestion["code_fix"].format(port=port)
                
                suggestions.append(suggestion)
        
        # Sort by confidence and severity
        suggestions.sort(key=lambda x: (
            {"high": 3, "medium": 2, "low": 1}[x["severity"]],
            x["confidence"]
        ), reverse=True)
        
        return suggestions
    
    def _get_error_context(self, log: str, match: re.Match) -> str:
        """Extract surrounding context from error log"""
        lines = log.split('\n')
        error_line = None
        
        # Find the line containing the match
        for i, line in enumerate(lines):
            if match.group(0) in line:
                error_line = i
                break
        
        if error_line is None:
            return match.group(0)
        
        # Return 2 lines before and after for context
        start = max(0, error_line - 2)
        end = min(len(lines), error_line + 3)
        context_lines = lines[start:end]
        
        return '\n'.join(context_lines)
    
    def record_deployment(self, success: bool, error_log: str = "", fixes_applied: List[str] = []):
        """Record deployment attempt for machine learning"""
        record = {
            "timestamp": datetime.now().isoformat(),
            "success": success,
            "error_log": error_log[:1000] if error_log else "",  # Truncate for storage
            "fixes_applied": fixes_applied,
            "suggestions": self.analyze_failure(error_log) if not success else []
        }
        
        self.deployment_history.append(record)
        self.save_history()
    
    def get_deployment_insights(self) -> Dict[str, Any]:
        """Generate insights from deployment history"""
        if not self.deployment_history:
            return {"message": "No deployment history yet"}
        
        total = len(self.deployment_history)
        successful = sum(1 for d in self.deployment_history if d["success"])
        
        # Find most common failure categories
        category_count = {}
        for deployment in self.deployment_history:
            if not deployment["success"]:
                for suggestion in deployment.get("suggestions", []):
                    cat = suggestion["category"]
                    category_count[cat] = category_count.get(cat, 0) + 1
        
        return {
            "total_deployments": total,
            "success_rate": f"{(successful/total)*100:.1f}%",
            "common_failures": dict(sorted(category_count.items(), key=lambda x: x[1], reverse=True)),
            "trend": "improving" if successful > (total - successful) else "needs_attention"
        }
    
    def generate_fix_script(self, suggestions: List[Dict[str, Any]]) -> str:
        """Generate executable fix script"""
        script_lines = [
            "#!/bin/bash",
            "# Auto-generated deployment fixes 🤖",
            "set -e",
            ""
        ]
        
        for i, suggestion in enumerate(suggestions):
            script_lines.extend([
                f"# Fix {i+1}: {suggestion['fix']}",
                f"echo '🔧 Applying fix: {suggestion['fix']}'",
                suggestion["code_fix"],
                ""
            ])
        
        return '\n'.join(script_lines)

def main():
    doctor = DeploymentDoctor()
    
    if len(sys.argv) < 2:
        print("Usage: python deployment-ai.py <command> [args]")
        print("Commands:")
        print("  analyze <error_log_file>  - Analyze deployment failure")
        print("  insights                  - Show deployment insights") 
        print("  record <success/failure> [log_file] - Record deployment")
        return
    
    command = sys.argv[1]
    
    if command == "analyze":
        if len(sys.argv) < 3:
            print("❌ Please provide error log file")
            return
            
        log_file = Path(sys.argv[2])
        if not log_file.exists():
            # Try reading from stdin
            error_log = sys.stdin.read()
        else:
            error_log = log_file.read_text()
        
        print("🤖 DeploymentAI Analysis Report")
        print("=" * 40)
        
        suggestions = doctor.analyze_failure(error_log)
        
        if not suggestions:
            print("✅ No known issues detected!")
            return
        
        for i, suggestion in enumerate(suggestions):
            print(f"\n🔍 Issue #{i+1} ({suggestion['severity']} severity)")
            print(f"Category: {suggestion['category']}")  
            print(f"Fix: {suggestion['fix']}")
            print(f"Code: {suggestion['code_fix']}")
            print(f"Confidence: {suggestion['confidence']*100:.0f}%")
            print(f"Context: {suggestion['error_context'][:100]}...")
        
        # Generate fix script
        fix_script = doctor.generate_fix_script(suggestions)
        with open("auto_fix.sh", "w") as f:
            f.write(fix_script)
        print(f"\n💾 Generated fix script: auto_fix.sh")
        print("Run: chmod +x auto_fix.sh && ./auto_fix.sh")
        
    elif command == "insights":
        insights = doctor.get_deployment_insights()
        print("📊 Deployment Insights")
        print("=" * 30)
        for key, value in insights.items():
            print(f"{key}: {value}")
            
    elif command == "record":
        if len(sys.argv) < 3:
            print("❌ Please specify success or failure")
            return
            
        success = sys.argv[2].lower() == "success"
        error_log = ""
        
        if len(sys.argv) > 3:
            log_file = Path(sys.argv[3])
            if log_file.exists():
                error_log = log_file.read_text()
        
        doctor.record_deployment(success, error_log)
        print(f"📝 Recorded {'successful' if success else 'failed'} deployment")

if __name__ == "__main__":
    main()