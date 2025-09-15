# MEVana UI Crates Analysis
Last updated: 2025-09-15

**Document Created**: August 14, 2025  
**Last Updated**: August 14, 2025  
**Status**: Active Development  

---

## 📋 Overview

The MEVana project now includes two dedicated UI crates that provide a modern web interface for monitoring and interacting with the MEV trading platform. These crates work together to create a real-time dashboard for tracking prices, swaps, and trading activities.

---

## 🏗️ UI Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           MEVana UI System                                  │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐        │
│  │   Kafka Topics  │    │  UI Gateway     │    │   Web Frontend  │        │
│  │                 │    │  (Rust/Axum)    │    │  (React/TS)     │        │
│  │ • prices.latest │───▶│ • WebSocket     │───▶│ • Real-time UI  │        │
│  │ • orca-swaps    │    │ • Kafka Bridge  │    │ • Price Charts  │        │
│  │ • MEV Data      │    │ • Data Stream   │    │ • Swap Monitor  │        │
│  └─────────────────┘    └─────────────────┘    └─────────────────┘        │
│           │                       │                       │                │
│           ▼                       ▼                       ▼                │
│  ┌─────────────────────────────────────────────────────────────────────┐  │
│  │                    Data Flow Architecture                            │  │
│  │  • Kafka → Rust Gateway → WebSocket → React Components              │  │
│  │  • Real-time price updates and swap monitoring                      │  │
│  │  • WebSocket-based live data streaming                              │  │
│  └─────────────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 🎯 UI Crates Breakdown

### 1. **ui-gateway** (Rust Backend)

**Purpose**: Real-time data gateway that bridges Kafka streams to WebSocket connections for the frontend.

**Technology Stack**:
- **Framework**: Axum (Rust web framework)
- **Async Runtime**: Tokio
- **Message Queue**: rdkafka (Kafka consumer)
- **Communication**: WebSocket (real-time bidirectional)
- **State Management**: Tokio broadcast channels

**Key Features**:
- **Kafka Integration**: Consumes from `prices.latest` and `orca-swaps.parsed` topics
- **WebSocket Endpoints**: 
  - `/ws/prices` - Real-time price data stream
  - `/ws/swaps` - Real-time swap transaction stream
  - `/ws/ping` - Health check endpoint
- **Real-time Streaming**: Broadcasts Kafka messages to connected WebSocket clients
- **Configurable**: Environment-based configuration for Kafka brokers and topics

**Architecture**:
```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Kafka Topics  │───▶│  Rust Gateway   │───▶│  WebSocket     │
│                 │    │                 │    │   Clients      │
│ • prices.latest │    │ • Consumer      │    │ • React App    │
│ • orca-swaps    │    │ • Broadcast     │    │ • Real-time    │
└─────────────────┘    │ • WebSocket     │    │   Updates      │
                       └─────────────────┘    └─────────────────┘
```

**Configuration**:
- `KAFKA_BROKER`: Kafka broker address (default: localhost:29092)
- `PRICES_TOPIC`: Prices topic name (default: prices.latest)
- `SWAPS_TOPIC`: Swaps topic name (default: orca-swaps.parsed)
- **Port**: 8081

---

### 2. **ui-web** (React Frontend)

**Purpose**: Modern web dashboard for real-time MEV monitoring and trading activity visualization.

**Technology Stack**:
- **Framework**: React 19 + TypeScript
- **Build Tool**: Vite (fast development and build)
- **Styling**: Tailwind CSS with custom design system
- **State Management**: Zustand (lightweight state store)
- **Charts**: Lightweight Charts (TradingView-compatible)
- **UI Components**: Custom component library with Framer Motion animations

**Key Features**:
- **Real-time Dashboard**: Live price updates and swap monitoring
- **Responsive Design**: Grid-based layout with mobile optimization
- **Dark Mode**: Custom CSS variables for theming
- **Component Architecture**: Modular, reusable UI components
- **WebSocket Integration**: Real-time data streaming from gateway

**Component Structure**:
```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              MEVana Dashboard                               │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐  │
│  │                           Header Bar                                 │  │
│  │  MEVana • Neon Carbon • WebSocket Status                            │  │
│  └─────────────────────────────────────────────────────────────────────┘  │
│                                                                             │
│  ┌─────────────────────────────┐  ┌─────────────────────────────────────┐  │
│  │        Prices Panel         │  │           Swaps Panel               │  │
│  │     (7/12 columns)          │  │         (5/12 columns)              │  │
│  │                             │  │                                     │  │
│  │  • Price Cards Grid         │  │  • Real-time Swap List              │  │
│  │  • Live Price Updates       │  │  • Transaction Details              │  │
│  │  • Market Data              │  │  • DEX Information                  │  │
│  └─────────────────────────────┘  └─────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Core Components**:
- **PricesPanel**: Grid layout of price cards showing live market data
- **SwapsPanel**: Real-time list of swap transactions with details
- **PriceCard**: Individual price display component
- **WsStatus**: WebSocket connection status indicator

**State Management**:
- **Zustand Store**: Centralized state for prices and swaps
- **WebSocket Hooks**: Custom hooks for real-time data management
- **Reactive Updates**: Automatic UI updates when new data arrives

---

## 🔄 Data Flow Architecture

### Real-time Data Pipeline

```
┌─────────────┐    ┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│  Solana     │───▶│  MEVana     │───▶│  Kafka     │───▶│  UI        │
│  Blockchain │    │  Streaming  │    │  Topics    │    │  Gateway   │
│             │    │  Services   │    │             │    │             │
└─────────────┘    └─────────────┘    └─────────────┘    └─────────────┘
                                                              │
                                                              ▼
┌─────────────┐    ┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│  React      │◀───│  WebSocket  │◀───│  Broadcast  │◀───│  Rust      │
│  Dashboard  │    │  Stream     │    │  Channels   │    │  Gateway   │
│             │    │             │    │             │    │             │
└─────────────┘    └─────────────┘    └─────────────┘    └─────────────┘
```

### WebSocket Message Flow

1. **Kafka Consumer**: Gateway consumes messages from Kafka topics
2. **Broadcast Channel**: Messages are broadcast to all subscribers
3. **WebSocket Stream**: Connected clients receive real-time updates
4. **React State**: Frontend updates state and re-renders components
5. **UI Updates**: Dashboard reflects latest data automatically

---

## 🚀 Development & Deployment

### Development Commands

**UI Gateway (Rust)**:
```bash
cd crates/ui-gateway
cargo run
```

**UI Web (React)**:
```bash
cd crates/ui-web
npm install
npm run dev          # Development server
npm run dev:mock     # Development with mock data
npm run build        # Production build
npm run preview      # Preview production build
```

### Environment Configuration

**UI Gateway**:
```bash
KAFKA_BROKER=localhost:29092
PRICES_TOPIC=prices.latest
SWAPS_TOPIC=orca-swaps.parsed
```

**UI Web**:
```bash
VITE_MOCK=1          # Enable mock data mode
```

### Port Configuration

- **UI Gateway**: `:8081` (WebSocket endpoints)
- **UI Web**: `:5173` (Vite dev server, configurable)

---

## 🎨 Design System

### Color Scheme

The UI uses a custom CSS variable system for consistent theming:

```css
:root {
  --bg: #0a0a0a;           /* Background */
  --panel: #1a1a1a;        /* Panel background */
  --panel-2: #2a2a2a;      /* Secondary panel */
  --text: #ffffff;          /* Primary text */
  --muted: #888888;         /* Muted text */
  --accent: #00ff88;        /* Primary accent */
  --accent-2: #0088ff;      /* Secondary accent */
  --up: #00ff88;            /* Positive/up */
  --down: #ff4444;          /* Negative/down */
  --warn: #ffaa00;          /* Warning */
}
```

### Component Patterns

- **Card-based Layout**: Consistent panel design with rounded corners
- **Grid System**: Responsive 12-column grid layout
- **Typography**: Clean, readable text with proper hierarchy
- **Spacing**: Consistent spacing using Tailwind's spacing scale
- **Animations**: Smooth transitions and micro-interactions

---

## 🔧 Technical Implementation

### WebSocket Integration

**Frontend Hook** (`useWS`):
- Manages WebSocket connections to gateway
- Handles connection state and reconnection
- Processes incoming messages and updates store

**Gateway Endpoints**:
- `/ws/prices`: Streams real-time price data
- `/ws/swaps`: Streams real-time swap transactions
- `/ws/ping`: Health check and connection testing

### State Management

**Zustand Store Structure**:
```typescript
interface Store {
  prices: Record<string, PriceData>
  swaps: SwapTransaction[]
  wsConnected: boolean
}
```

**Real-time Updates**:
- Automatic state updates when WebSocket messages arrive
- Optimistic UI updates for better user experience
- Efficient re-rendering with React's reconciliation

---

## 📊 Current Features

### ✅ Implemented

- **Real-time Price Display**: Live price updates from Kafka streams
- **Swap Transaction Monitor**: Real-time swap transaction list
- **WebSocket Status**: Connection health monitoring
- **Responsive Layout**: Mobile and desktop optimized design
- **Dark Theme**: Modern, professional appearance
- **Component Architecture**: Modular, maintainable code structure

### 🚧 In Development

- **Price Charts**: Historical price visualization
- **Advanced Filtering**: Search and filter capabilities
- **Trading Interface**: Direct trading functionality
- **Analytics Dashboard**: MEV opportunity analysis
- **User Authentication**: Secure access control

---

## 🔮 Future Roadmap

### Phase 1: Core Dashboard (Current)
- ✅ Basic price and swap monitoring
- ✅ Real-time data streaming
- ✅ Responsive design

### Phase 2: Enhanced Analytics
- 📊 Advanced charting and technical analysis
- 📈 Historical data visualization
- 🔍 MEV opportunity detection and alerts

### Phase 3: Trading Interface
- 💰 Direct trading execution
- 📋 Order management
- 🎯 Strategy automation

### Phase 4: Advanced Features
- 🤖 AI-powered insights
- 📱 Mobile application
- 🔐 Advanced security and authentication

---

## 🛠️ Development Guidelines

### Code Quality

- **TypeScript**: Strict type checking enabled
- **ESLint**: Comprehensive linting rules
- **Prettier**: Consistent code formatting
- **Component Testing**: Unit tests for critical components

### Performance

- **Virtual Scrolling**: For large data sets
- **Memoization**: React.memo and useMemo for expensive operations
- **Lazy Loading**: Code splitting and dynamic imports
- **WebSocket Optimization**: Efficient message handling

### Accessibility

- **ARIA Labels**: Proper screen reader support
- **Keyboard Navigation**: Full keyboard accessibility
- **Color Contrast**: WCAG compliant color schemes
- **Responsive Design**: Mobile-first approach

---

## 📚 Related Documentation

- [MEVana Architecture Overview](../ARCHITECTURE_OVERVIEW.md)
- [Crate Inventory](../CRATE_INVENTORY.md)
- [Docker Setup](../deployment/docker-setup.md)
- [Development Guide](../development/getting-started.md)

---

## 🔗 External Resources

- **React 19**: [Official Documentation](https://react.dev/)
- **Vite**: [Build Tool Documentation](https://vitejs.dev/)
- **Tailwind CSS**: [Utility-First CSS Framework](https://tailwindcss.com/)
- **Axum**: [Rust Web Framework](https://github.com/tokio-rs/axum)
- **Zustand**: [Lightweight State Management](https://github.com/pmndrs/zustand)

---

*This document provides a comprehensive overview of the MEVana UI system. For specific implementation details, refer to the individual crate documentation and source code.*
