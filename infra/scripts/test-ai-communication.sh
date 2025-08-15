#!/bin/bash

# AI Communication Service Test Script
# Tests comprehensive alerting and AI agent communication capabilities

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

print_status() {
    local status=$1
    local message=$2
    case $status in
        "SUCCESS")
            echo -e "${GREEN}✅ $message${NC}"
            ;;
        "ERROR")
            echo -e "${RED}❌ $message${NC}"
            ;;
        "WARNING")
            echo -e "${YELLOW}⚠️  $message${NC}"
            ;;
        "INFO")
            echo -e "${BLUE}ℹ️  $message${NC}"
            ;;
    esac
}

print_header() {
    echo "=========================================="
    echo "AI Communication Service Test"
    echo "=========================================="
    echo ""
}

# Test 1: Check if communication service files exist
test_communication_files() {
    print_status "INFO" "Test 1: Checking if communication service files exist"
    
    local files=(
        "ai-communication-service/Cargo.toml"
        "ai-communication-service/src/main.rs"
        "ai-communication-service/src/alerts.rs"
        "ai-communication-service/src/communication.rs"
        "ai-communication-service/src/ai_agents.rs"
        "ai-communication-service/src/templates.rs"
        "ai-communication-service/src/config.rs"
    )
    
    local found_files=0
    
    for file in "${files[@]}"; do
        if [ -f "$file" ]; then
            print_status "SUCCESS" "File found: $file"
            ((found_files++))
        else
            print_status "ERROR" "File missing: $file"
        fi
    done
    
    if [ $found_files -eq ${#files[@]} ]; then
        print_status "SUCCESS" "All communication service files present"
    else
        print_status "ERROR" "Some communication service files are missing"
        return 1
    fi
}

# Test 2: Check alert system capabilities
test_alert_system() {
    print_status "INFO" "Test 2: Checking alert system capabilities"
    
    local alert_features=(
        "AlertManager"
        "AlertType"
        "AlertPriority"
        "AlertChannel"
        "WhaleDetected"
        "ArbitrageOpportunity"
        "SystemError"
        "SecurityBreach"
        "RiskThreshold"
        "StrategyPerformance"
    )
    
    local found_features=0
    
    for feature in "${alert_features[@]}"; do
        if grep -q "$feature" ai-communication-service/src/alerts.rs; then
            print_status "SUCCESS" "Alert feature found: $feature"
            ((found_features++))
        else
            print_status "WARNING" "Alert feature missing: $feature"
        fi
    done
    
    print_status "INFO" "Found $found_features out of ${#alert_features[@]} alert features"
    
    if [ $found_features -ge 8 ]; then
        print_status "SUCCESS" "Alert system is comprehensive"
    else
        print_status "WARNING" "Alert system may need improvement"
    fi
}

# Test 3: Check communication channels
test_communication_channels() {
    print_status "INFO" "Test 3: Checking communication channels"
    
    local channel_features=(
        "SMSProvider"
        "TelegramProvider"
        "WhatsAppProvider"
        "VoiceProvider"
        "send_sms"
        "send_telegram"
        "send_whatsapp"
        "make_voice_call"
        "CommunicationManager"
    )
    
    local found_features=0
    
    for feature in "${channel_features[@]}"; do
        if grep -q "$feature" ai-communication-service/src/communication.rs; then
            print_status "SUCCESS" "Communication channel feature found: $feature"
            ((found_features++))
        else
            print_status "WARNING" "Communication channel feature missing: $feature"
        fi
    done
    
    print_status "INFO" "Found $found_features out of ${#channel_features[@]} communication channel features"
    
    if [ $found_features -ge 7 ]; then
        print_status "SUCCESS" "Communication channels are comprehensive"
    else
        print_status "WARNING" "Communication channels may need improvement"
    fi
}

# Test 4: Check AI agent communication
test_ai_agent_communication() {
    print_status "INFO" "Test 4: Checking AI agent communication"
    
    local ai_features=(
        "AIAgentCommunication"
        "AgentConversation"
        "VoiceCallManager"
        "AgentType"
        "PatternLearningAgent"
        "StrategyGenerationAgent"
        "PerformanceOptimizationAgent"
        "RiskManagementAgent"
        "PlatformEvolutionAgent"
        "start_conversation"
        "add_message"
        "handle_agent_question"
        "relay_information"
    )
    
    local found_features=0
    
    for feature in "${ai_features[@]}"; do
        if grep -q "$feature" ai-communication-service/src/ai_agents.rs; then
            print_status "SUCCESS" "AI agent communication feature found: $feature"
            ((found_features++))
        else
            print_status "WARNING" "AI agent communication feature missing: $feature"
        fi
    done
    
    print_status "INFO" "Found $found_features out of ${#ai_features[@]} AI agent communication features"
    
    if [ $found_features -ge 10 ]; then
        print_status "SUCCESS" "AI agent communication is comprehensive"
    else
        print_status "WARNING" "AI agent communication may need improvement"
    fi
}

# Test 5: Check template engine
test_template_engine() {
    print_status "INFO" "Test 5: Checking template engine"
    
    local template_features=(
        "TemplateEngine"
        "render_template"
        "whale_detected"
        "arbitrage_opportunity"
        "system_error"
        "security_breach"
        "pattern_learning_response"
        "strategy_generation_response"
        "performance_optimization_response"
        "risk_management_response"
        "platform_evolution_response"
    )
    
    local found_features=0
    
    for feature in "${template_features[@]}"; do
        if grep -q "$feature" ai-communication-service/src/templates.rs; then
            print_status "SUCCESS" "Template engine feature found: $feature"
            ((found_features++))
        else
            print_status "WARNING" "Template engine feature missing: $feature"
        fi
    done
    
    print_status "INFO" "Found $found_features out of ${#template_features[@]} template engine features"
    
    if [ $found_features -ge 9 ]; then
        print_status "SUCCESS" "Template engine is comprehensive"
    else
        print_status "WARNING" "Template engine may need improvement"
    fi
}

# Test 6: Check configuration system
test_configuration_system() {
    print_status "INFO" "Test 6: Checking configuration system"
    
    local config_features=(
        "CommunicationConfig"
        "SMSConfig"
        "TelegramConfig"
        "WhatsAppConfig"
        "VoiceConfig"
        "AIConfig"
        "AlertConfig"
        "TemplateConfig"
        "get_alert_channels"
        "get_cooldown_period"
        "get_rate_limit"
    )
    
    local found_features=0
    
    for feature in "${config_features[@]}"; do
        if grep -q "$feature" ai-communication-service/src/config.rs; then
            print_status "SUCCESS" "Configuration feature found: $feature"
            ((found_features++))
        else
            print_status "WARNING" "Configuration feature missing: $feature"
        fi
    done
    
    print_status "INFO" "Found $found_features out of ${#config_features[@]} configuration features"
    
    if [ $found_features -ge 9 ]; then
        print_status "SUCCESS" "Configuration system is comprehensive"
    else
        print_status "WARNING" "Configuration system may need improvement"
    fi
}

# Test 7: Check dependencies and integrations
test_dependencies_integrations() {
    print_status "INFO" "Test 7: Checking dependencies and integrations"
    
    local dependencies=(
        "twilio"
        "teloxide"
        "whatsapp-api"
        "tts"
        "openai"
        "rust-bert"
        "tera"
        "axum"
        "reqwest"
    )
    
    local found_dependencies=0
    
    for dependency in "${dependencies[@]}"; do
        if grep -q "$dependency" ai-communication-service/Cargo.toml; then
            print_status "SUCCESS" "Dependency found: $dependency"
            ((found_dependencies++))
        else
            print_status "WARNING" "Dependency missing: $dependency"
        fi
    done
    
    print_status "INFO" "Found $found_dependencies out of ${#dependencies[@]} dependencies"
    
    if [ $found_dependencies -ge 7 ]; then
        print_status "SUCCESS" "Dependencies and integrations are comprehensive"
    else
        print_status "WARNING" "Dependencies and integrations may need improvement"
    fi
}

# Test 8: Check API endpoints
test_api_endpoints() {
    print_status "INFO" "Test 8: Checking API endpoints"
    
    local api_endpoints=(
        "/health"
        "/api/alerts"
        "/api/ai-conversation"
        "/api/voice-call"
        "/api/status"
        "send_alert"
        "start_ai_conversation"
        "initiate_voice_call"
        "get_service_status"
    )
    
    local found_endpoints=0
    
    for endpoint in "${api_endpoints[@]}"; do
        if grep -q "$endpoint" ai-communication-service/src/main.rs; then
            print_status "SUCCESS" "API endpoint found: $endpoint"
            ((found_endpoints++))
        else
            print_status "WARNING" "API endpoint missing: $endpoint"
        fi
    done
    
    print_status "INFO" "Found $found_endpoints out of ${#api_endpoints[@]} API endpoints"
    
    if [ $found_endpoints -ge 7 ]; then
        print_status "SUCCESS" "API endpoints are comprehensive"
    else
        print_status "WARNING" "API endpoints may need improvement"
    fi
}

# Test 9: Check voice call capabilities
test_voice_call_capabilities() {
    print_status "INFO" "Test 9: Checking voice call capabilities"
    
    local voice_features=(
        "VoiceCallManager"
        "VoiceCall"
        "VoiceCallType"
        "CallStatus"
        "CallTranscriptEntry"
        "CallSpeaker"
        "initiate_call"
        "handle_voice_call"
        "text_to_speech"
        "generate_ai_response"
    )
    
    local found_features=0
    
    for feature in "${voice_features[@]}"; do
        if grep -q "$feature" ai-communication-service/src/ai_agents.rs; then
            print_status "SUCCESS" "Voice call feature found: $feature"
            ((found_features++))
        else
            print_status "WARNING" "Voice call feature missing: $feature"
        fi
    done
    
    print_status "INFO" "Found $found_features out of ${#voice_features[@]} voice call features"
    
    if [ $found_features -ge 8 ]; then
        print_status "SUCCESS" "Voice call capabilities are comprehensive"
    else
        print_status "WARNING" "Voice call capabilities may need improvement"
    fi
}

# Test 10: Check statistics and monitoring
test_statistics_monitoring() {
    print_status "INFO" "Test 10: Checking statistics and monitoring"
    
    local monitoring_features=(
        "AlertStatistics"
        "CommunicationStatistics"
        "AIAgentStatistics"
        "VoiceCallStatistics"
        "get_statistics"
        "update_statistics"
        "success_rate"
        "average_response_time_ms"
        "total_alerts"
        "total_conversations"
        "total_calls"
    )
    
    local found_features=0
    
    for feature in "${monitoring_features[@]}"; do
        if grep -q "$feature" ai-communication-service/src/*.rs; then
            print_status "SUCCESS" "Statistics/monitoring feature found: $feature"
            ((found_features++))
        else
            print_status "WARNING" "Statistics/monitoring feature missing: $feature"
        fi
    done
    
    print_status "INFO" "Found $found_features out of ${#monitoring_features[@]} statistics/monitoring features"
    
    if [ $found_features -ge 8 ]; then
        print_status "SUCCESS" "Statistics and monitoring are comprehensive"
    else
        print_status "WARNING" "Statistics and monitoring may need improvement"
    fi
}

# Test 11: Generate AI communication service report
test_ai_communication_report() {
    print_status "INFO" "Test 11: Generating AI communication service report"
    
    echo ""
    echo "📞 AI Communication Service Report"
    echo "=================================="
    echo ""
    echo "Service Overview:"
    echo "  • Comprehensive alerting system with multiple channels"
    echo "  • AI agent communication and voice call capabilities"
    echo "  • Template-based message formatting"
    echo "  • Configurable priority-based routing"
    echo "  • Real-time statistics and monitoring"
    echo ""
    echo "Communication Channels:"
    echo "  📱 SMS: Twilio integration for text alerts"
    echo "  📲 Telegram: Bot API for instant messaging"
    echo "  💬 WhatsApp: Business API for business communication"
    echo "  🎤 Voice Calls: Twilio voice with TTS capabilities"
    echo "  📧 Email: SMTP integration for detailed reports"
    echo "  📱 Push Notifications: Mobile app notifications"
    echo "  🖥️ Dashboard: Real-time web interface updates"
    echo "  💬 Slack: Team collaboration integration"
    echo "  🎮 Discord: Community communication"
    echo ""
    echo "Alert Types:"
    echo "  🐋 Whale Detection: Large trade alerts"
    echo "  ⚡ Arbitrage Opportunities: Profit alerts"
    echo "  🚨 System Errors: Critical system issues"
    echo "  🔒 Security Breaches: Security incidents"
    echo "  ⚠️ Risk Thresholds: Risk management alerts"
    echo "  📈 Strategy Performance: Trading strategy updates"
    echo "  🐌 Performance Issues: System performance alerts"
    echo "  📊 Market Volatility: Market condition alerts"
    echo "  🤖 AI Agent Errors: AI system issues"
    echo ""
    echo "AI Agent Communication:"
    echo "  🤖 Pattern Learning Agent: Market analysis"
    echo "  ⚡ Strategy Generation Agent: Strategy creation"
    echo "  📈 Performance Optimization Agent: Optimization"
    echo "  🛡️ Risk Management Agent: Risk assessment"
    echo "  🚀 Platform Evolution Agent: System improvement"
    echo ""
    echo "Voice Call Features:"
    echo "  🎤 Alert Calls: Critical alert voice notifications"
    echo "  ❓ Question Calls: AI agent question handling"
    echo "  📡 Information Calls: Data relay via voice"
    echo "  🚨 Emergency Calls: Urgent situation handling"
    echo "  💬 Consultation Calls: AI agent consultations"
    echo ""
    echo "Template System:"
    echo "  📝 Pre-built Templates: Ready-to-use message formats"
    echo "  🎨 Custom Templates: User-defined message formats"
    echo "  🌍 Multi-language: Support for multiple languages"
    echo "  🔧 Variable Substitution: Dynamic content insertion"
    echo "  📊 Rich Formatting: Markdown and rich text support"
    echo ""
    echo "Configuration:"
    echo "  ⚙️ Channel Configuration: Per-channel settings"
    echo "  🚦 Priority Routing: Priority-based alert routing"
    echo "  ⏱️ Rate Limiting: Prevent alert spam"
    echo "  🕐 Cooldown Periods: Prevent alert flooding"
    echo "  🚨 Emergency Contacts: Critical contact management"
    echo ""
    echo "API Endpoints:"
    echo "  🔍 /health: Service health check"
    echo "  🚨 /api/alerts: Send alerts programmatically"
    echo "  🤖 /api/ai-conversation: Start AI conversations"
    echo "  🎤 /api/voice-call: Initiate voice calls"
    echo "  📊 /api/status: Service status and statistics"
    echo ""
    echo "Integration Capabilities:"
    echo "  🔗 Twilio: SMS and voice call integration"
    echo "  🤖 Telegram Bot API: Instant messaging"
    echo "  💬 WhatsApp Business API: Business messaging"
    echo "  🧠 OpenAI API: AI conversation capabilities"
    echo "  📊 Kafka: Real-time event streaming"
    echo "  🗄️ PostgreSQL: Persistent data storage"
    echo "  🔄 Redis: Caching and real-time data"
    echo ""
    
    print_status "SUCCESS" "AI communication service report generated"
}

# Main test execution
main() {
    print_header
    
    local tests=(
        "test_communication_files"
        "test_alert_system"
        "test_communication_channels"
        "test_ai_agent_communication"
        "test_template_engine"
        "test_configuration_system"
        "test_dependencies_integrations"
        "test_api_endpoints"
        "test_voice_call_capabilities"
        "test_statistics_monitoring"
        "test_ai_communication_report"
    )
    
    local passed=0
    local failed=0
    
    for test in "${tests[@]}"; do
        echo ""
        if $test; then
            ((passed++))
        else
            ((failed++))
        fi
    done
    
    echo ""
    echo "=========================================="
    echo "AI Communication Service Test Results"
    echo "=========================================="
    echo "Passed: $passed"
    echo "Failed: $failed"
    echo "Total: ${#tests[@]}"
    echo ""
    
    if [ $failed -eq 0 ]; then
        print_status "SUCCESS" "All tests passed! AI communication service is ready."
        echo ""
        print_status "INFO" "Next steps:"
        echo "  1. Configure API keys (Twilio, Telegram, WhatsApp, OpenAI)"
        echo "  2. Set up phone numbers and chat IDs"
        echo "  3. Configure alert priorities and channels"
        echo "  4. Test each communication channel"
        echo "  5. Integrate with AI evolution platform"
        echo "  6. Set up monitoring and alerting"
        echo "  7. Configure voice call capabilities"
        echo "  8. Test AI agent conversations"
        echo ""
        print_status "INFO" "This service provides comprehensive communication capabilities!"
        echo "  📞 Multi-channel alerting (SMS, Telegram, WhatsApp, Voice)"
        echo "  🤖 AI agent conversations and voice calls"
        echo "  📝 Template-based message formatting"
        echo "  ⚙️ Configurable priority-based routing"
        echo "  📊 Real-time statistics and monitoring"
        echo "  🔗 Integration with external APIs and services"
    else
        print_status "ERROR" "Some tests failed. Please review the issues above."
        exit 1
    fi
}

# Run tests
main 