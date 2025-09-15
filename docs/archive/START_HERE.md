# 🚀 START HERE - MEVana Project Entry Point
Last updated: 2025-09-15

**Last Updated**: 2024-08-19 15:30 UTC  
**Version**: 1.0.0  
**Critical**: READ THIS FIRST before working on this project

## **Project Overview**
MEVana is a high-frequency arbitrage trading system targeting <10ms latency from signal to execution. We're implementing a hybrid strategy combining Geyser streaming (speed) with Shyft APIs (robustness).

## 🔑 **Critical Information for AI Agents**

### **Current Status (August 19, 2024)**
- **Phase**: Hybrid Shyft Strategy Implementation
- **Main Problem**: TickArray deserialization failing with "Invalid bool representation"
- **Solution**: Pivoting to Shyft APIs for data validation and enrichment
- **Goal**: Maintain <10ms speed while improving data quality and reliability

### **Key Credentials & Endpoints**
- **Shyft API Key**: `fWrVh_mDygK358FC` (CONFIDENTIAL)
- **Network**: `mainnet-beta`
- **RPC**: `https://rpc.shyft.to?api_key=fWrVh_mDygK358FC`
- **GraphQL**: `https://programs.shyft.to/v0/graphql/?api_key=fWrVh_mDygK358FC&network=mainnet-beta`

### **Immediate Next Steps**
1. **Test Shyft APIs** using `scripts/test-shyft-apis.sh`
2. **Validate data quality** compared to current TickArray approach
3. **Design Redis cache schema** for hybrid data pipeline
4. **Implement hybrid strategy** maintaining <10ms speed

---
**Remember**: This project is about building a PROFITABLE arbitrage trading system. Speed and reliability are everything!
