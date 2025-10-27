<p align="center">
   <a href="https://flow-like.com" target="_blank">
      <picture>
         <source width=200 srcset="./apps/desktop/public/app-logo-light.webp" media="(prefers-color-scheme: dark)">
         <img width=200 src="./apps/desktop/public/app-logo.webp" alt="Icon">
      </picture>
   </a>
</p>

<div align="center">

[![Codacy Badge](https://app.codacy.com/project/badge/Grade/244d2db2a84f4e79b64d984639a2b18f)](https://app.codacy.com/gh/TM9657/flow-like/dashboard?utm_source=gh&utm_medium=referral&utm_content=&utm_campaign=Badge_grade)
![Discord](https://img.shields.io/discord/673169081704120334)
[![FOSSA Status](https://app.fossa.com/api/projects/custom%2B49014%2Fflow-like.svg?type=small)](https://app.fossa.com/projects/custom%2B49014%2Fflow-like?ref=badge_small)
[![Website](https://img.shields.io/badge/website-flow--like.com-0a7cff?logo=google-chrome&logoColor=white)](https://flow-like.com)
[![Docs](https://img.shields.io/badge/docs-docs.flow--like.com-0a7cff?logo=readthedocs&logoColor=white)](https://docs.flow-like.com)
[![Download](https://img.shields.io/badge/download-Desktop%20App-28a745?logo=tauri&logoColor=white)](https://flow-like.com/download)

</div>

<a name="Headline"></a>

<h1 align="center">Flow-Like: Automate Your Work — See the Full Data Story</h1>

<p align="center">
  <em>Any flow you like.</em>
</p>

<br>

<div align="center">

**🔒 Private by Default** • **⚡ Fast & Reliable** • **🧩 Drag-and-Drop Blocks** • **☁️ Works Solo or at Team Scale**

</div>

<br>

Flow-Like is a **visual workflow automation platform** that shows you not just *what* happens, but *why*. Build automated workflows with drag-and-drop blocks and get a clear record of where data came from, what changed, and what came out — **no black boxes, no guesswork**. Perfect for workflow automation, business process automation, data integration, and AI-powered workflows.

<p align="center">
  <img width="800" src="./assets/recording.gif" alt="Flow-Like Visual Workflow Studio in Action">
</p>

<div align="center">

**[⭐ Star us on GitHub](https://github.com/TM9657/flow-like/stargazers)** • **[📖 Read the Docs](https://docs.flow-like.com)** • **[💬 Join Discord](https://discord.com/invite/mdBA9kMjFJ)**

</div>

---

## 📖 Table of Contents

- [Why Choose Flow-Like?](#why-choose-flow-like-for-workflow-automation)
- [What Makes Us Different](#-what-makes-flow-like-different-from-other-workflow-automation-tools)
- [Comparison Table](#-flow-like-vs-traditional-workflow-tools)
- [White-Label & Customization](#-white-label--customization)
- [Quick Start](#-quick-start-with-flow-like)
- [Ecosystem](#-flow-like-workflow-automation-ecosystem)
  - [Visual Studio](#-visual-workflow-studio)
  - [Node Catalog](#-rich-node-catalog-for-workflow-automation)
  - [AI-Powered Workflows](#-ai-powered-workflow-automation)
  - [Apps & Templates](#-workflow-apps--templates)
- [For Every Role](#-workflow-automation-for-every-role)
- [Three Perspectives](#️-business-process-automation-one-process-three-perspectives)
- [Use Cases](#-use-cases--examples)
- [Build From Source](#️-build-flow-like-from-source)
- [Gallery](#-screenshots--gallery)
- [FAQ](#-frequently-asked-questions)
- [Community](#-join-the-flow-like-community)
- [Project Stats](#-project-stats)

---

### Why Choose Flow-Like for Workflow Automation?

🚀 **Fully Typed Workflows** — Type-safe data flows ready for enterprise-scale projects
🦀 **Built on Rust** — High-performance workflow engine with uncompromising speed and safety
🎯 **Zero-to-Prod** — The workflow you design is production-ready — no rewrites needed
🤖 **AI-Powered Automation** — Seamlessly integrate LLMs, ML models, and traditional logic
🌐 **Deploy Anywhere** — Start offline, go online, deploy to Edge/Cloud/On-prem with one click
🎨 **Fully Customizable** — White-label ready with custom themes, branding, and SSO
🏢 **Enterprise Ready** — Role-based access, compliance, audit trails, and process governance
📜 **Source Available** — BSL license: likely free for your use case

<br>

## 🎯 What Makes Flow-Like Different from Other Workflow Automation Tools

### The Challenge with Traditional Workflow Automation
Most workflow automation tools show a green checkmark and move on. You're left guessing where the data came from, what got filtered or transformed, and why the result looks the way it does. Traditional tools lack the transparency and type safety needed for complex enterprise workflows.

### Our Solution: Typed Workflows with Complete Data Trails
In Flow-Like, workflows are **fully typed** — they don't just describe *what happens*, but also *what data flows where* and *why*. Every workflow automation includes complete lineage tracking and audit trails.

<table>
<tr>
<td width="33%">

**🔍 Data Origins**
See exactly where each piece of data came from — the email address, the file, the API response.

</td>
<td width="33%">

**⚙️ Transformations**
Every validation, enrichment, and reformatting step is visible and traceable.

</td>
<td width="33%">

**📋 Clear Contracts**
Type-safe input/output definitions prevent runtime surprises and breaking changes.

</td>
</tr>
</table>

**Result:** Explainable workflows that stay shippable as they evolve — from prototype to production, with confidence.

<br>

## 📊 Flow-Like vs Traditional Workflow Tools

<table>
<tr>
<th width="25%">Feature</th>
<th width="25%">Flow-Like</th>
<th width="25%">Traditional Tools</th>
<th width="25%">Why It Matters</th>
</tr>
<tr>
<td><strong>Type Safety</strong></td>
<td>✅ Fully typed workflows</td>
<td>❌ Runtime-only validation</td>
<td>Catch errors before deployment, not in production</td>
</tr>
<tr>
<td><strong>Data Lineage</strong></td>
<td>✅ Complete audit trail</td>
<td>⚠️ Limited or none</td>
<td>Debug issues faster, meet compliance requirements</td>
</tr>
<tr>
<td><strong>Deployment</strong></td>
<td>✅ Local, Edge, Cloud, On-prem</td>
<td>⚠️ Usually cloud-only</td>
<td>Control where your data lives, work offline</td>
</tr>
<tr>
<td><strong>Performance</strong></td>
<td>✅ Rust-based, native speed</td>
<td>⚠️ Interpreted, slower</td>
<td>Process more data, lower infrastructure costs</td>
</tr>
<tr>
<td><strong>White-Label</strong></td>
<td>✅ Full customization, embed anywhere</td>
<td>❌ Branded UI only</td>
<td>Build your own product on top</td>
</tr>
<tr>
<td><strong>Offline Work</strong></td>
<td>✅ Full offline capability</td>
<td>❌ Requires internet</td>
<td>Work in secure/air-gapped environments</td>
</tr>
<tr>
<td><strong>Open Source</strong></td>
<td>✅ Source available (BSL)</td>
<td>❌ Proprietary</td>
<td>No vendor lock-in, transparent codebase</td>
</tr>
<tr>
<td><strong>Enterprise Features</strong></td>
<td>✅ RBAC, compliance, audit trails</td>
<td>⚠️ Enterprise tier only</td>
<td>Built-in governance from day one</td>
</tr>
<tr>
<td><strong>Business Process Views</strong></td>
<td>✅ Process, Data, Execution views</td>
<td>❌ Single view only</td>
<td>Align technical and business teams</td>
</tr>
</table>

<br>

## 🎨 White-Label & Customization

**Embed Flow-Like in Your Product** — Drop the visual workflow editor into your application, or run the engine behind the scenes. Your logo, your colors, your brand.

### Customization Features

- 🎭 **Custom Themes** — Pre-built themes (Catppuccin, Cosmic Night, Neo-Brutalism, Soft Pop, Doom) or create your own
- 🧪 **Design Tokens & CSS Variables** — Map your brand palette with instant dark/light mode support
- 🏢 **SSO & Identity** — OIDC/JWT integration with scoped secrets per tenant or app
- 📊 **Usage Metering** — Built-in per-tenant quotas, events tracking, and audit trails
- 🔌 **SDKs & APIs** — Control workflows programmatically via REST API and SDKs
- 🖼️ **Your Branding** — Replace logos, customize UI elements, and maintain your brand identity

**Perfect for:** SaaS platforms, internal tools, client portals, and embedded automation solutions.

<br>

## 🚀 Quick Start with Flow-Like

<div align="center">

| 💻 Desktop App | ☁️ Cloud | ⚙️ From Source |
|:---:|:---:|:---:|
| [**Download Now**](https://github.com/TM9657/flow-like/releases)<br>Run locally on macOS, Windows, or Linux | [**Try Online**](https://flow-like.com/)<br>Coming soon | [**Build Yourself**](#build-from-source)<br>Latest features |

</div>

<br>

## 🌐 Flow-Like Workflow Automation Ecosystem

### 🎨 Visual Workflow Studio
Our innovative, **no-code workflow builder IDE** for creating automated workflows. Connect nodes with smart predictions, collapse complex logic into clean abstractions, and trace every execution with inline feedback.

<p align="center"><img width="800" src="./assets/recording.gif" alt="Visual Studio in Action"></p>

**Features:**
- 🎯 **Smart Wiring** — Pins know what they accept; miswired connections surface immediately
- 📊 **Inline Feedback** — See inputs, outputs, and timings at each step
- 🔄 **Live Validation** — Fix mistakes as you go, before they ship
- 📸 **Snapshots** — Reproduce issues and compare runs with saved states

---

### 🧩 Rich Node Catalog for Workflow Automation
Build automated workflows from **100+ pre-built execution nodes** — from data transformation and database operations to AI models and higher-order agent nodes.

**Workflow Node Categories:**
- 🔗 **APIs & Webhooks** — Connect to any REST API, GraphQL endpoint, or webhook
- 🗄️ **Databases & Storage** — SQL, NoSQL, object storage, and more
- 📁 **Files & Processing** — Excel, CSV, PDF, images, and document processing
- 🤖 **AI & Computer Vision** — LLMs, image recognition, object detection, embeddings
- 📨 **Messaging & Queues** — Email, Slack, Discord, Kafka, RabbitMQ
- 🌐 **Devices & Sensors** — IoT integration and real-time data processing
- 🔄 **Logic & Control** — Branching, loops, conditions, and error handling
- 🔐 **Security & Auth** — Authentication, encryption, and access control

[📄 Explore the Full Node Catalog →](https://docs.flow-like.com/)

---

### 🤖 AI-Powered Workflow Automation
Download and run **LLMs, VLMs (Vision Language Models), Deep Learning models**, and embeddings — locally or in the cloud. Build intelligent, AI-powered workflows with context-aware automation nodes.

<p align="center"><img width="800" src="https://cdn.flow-like.com/website/SelectYourModel.webp" alt="AI Model Catalog"></p>

---

### 📦 Workflow Apps & Templates
Create **shareable workflow applications** with built-in storage and automation logic. Run them offline, online, locally, or in the cloud. Browse our public workflow template store or share your own automation templates with the community.

<p align="center"><img width="800" src="https://cdn.flow-like.com/website/CreateApp.webp" alt="Create Apps"></p>
<p align="center"><img width="800" src="https://cdn.flow-like.com/website/Store.webp" alt="Template Store"></p>


<br>

## 💡 Workflow Automation for Every Role

<table>
<tr>
<td width="33%" valign="top">

### 👨‍💻 For Developers & Engineers

✅ **Type-Safe Development** — Build workflows with type-safe data contracts
✅ **Extensible Platform** — Create custom nodes and integrations
✅ **Workflow Templates** — Share and reuse automation patterns
✅ **Git-Based Version Control** — Track every workflow change
✅ **Deploy Anywhere** — Edge, Cloud, On-prem, or Local environments
✅ **Source Available** — Transparent codebase, likely free for your needs

</td>
<td width="33%" valign="top">

### 📊 For Business & Analysts

✅ **No-Code Automation** — Build workflows without programming knowledge
✅ **Business Process Modeling** — Visualize and automate business logic
✅ **Multiple Views** — Process, Data, and Technical perspectives
✅ **Team Collaboration** — Role-based access and approval workflows
✅ **Change Tracking** — Audit trails for compliance and reviews

</td>
<td width="33%" valign="top">

### 🏢 For IT & Operations Teams

✅ **Enterprise Governance** — Centralized platform for compliance monitoring
✅ **Role-Based Access Control** — Fine-grained permissions and team management
✅ **Production-Ready** — Validated workflows from POC to production
✅ **High Performance** — Rust-based engine for predictable speed
✅ **Complete Audit Trails** — Every step logged for compliance
✅ **Process Compliance** — Built-in governance, approvals, and policy enforcement
✅ **Air-Gap Deployment** — Run fully offline in secure environments

</td>
</tr>
</table>

## 🎛️ Business Process Automation: One Process, Three Perspectives

Flow-Like goes beyond simple task automation — it's built for **end-to-end business process orchestration** where every role sees the same truth in their own language.

<div align="center">

| 📋 Process View | 🔄 Data View | ⚙️ Execution View |
|:---:|:---:|:---:|
| **Who does what, when, and why**<br>Plain-English story for managers | **What came in, what changed, what went out**<br>Data transformations and lineage | **How the system runs it**<br>Technical implementation for IT |

</div>

### Hierarchical Process Modeling
- 📊 **Executive View** — High-level business processes for stakeholders
- 🔍 **Technical View** — Detailed implementation one layer below
- 🌐 **Cross-Team Workflows** — Manage enterprise-wide automation without silos

**Result:** Business logic and technical execution stay aligned, from strategy to deployment. Perfect for process mining, business process management (BPM), and workflow orchestration.

<br>

## 📦 Use Cases & Examples

Flow-Like powers automation across industries and use cases:

- 📧 **Email Automation** — Smart routing, filtering, and response automation
- 📊 **Data Integration** — ETL pipelines, data transformation, and synchronization
- 🤖 **AI Workflows** — Document processing, content generation, image analysis
- 🏢 **Business Process Automation** — Approval workflows, document routing, compliance
- 🔄 **API Integration** — Connect multiple services and automate data flows
- 📈 **Analytics Pipelines** — Data collection, processing, and visualization
- 🛒 **E-commerce Automation** — Order processing, inventory management, notifications
- 🎯 **Marketing Automation** — Campaign management, lead scoring, personalization
- 🔐 **Security & Compliance** — Automated audits, access reviews, incident response
- 🌐 **IoT & Edge Computing** — Device management, data aggregation, real-time processing

<br>

## ⚙️ Build Flow-Like From Source

Want the latest workflow automation features? Build the desktop app yourself:

```bash
# 1. Install Prerequisites
# - Rust: https://www.rust-lang.org/tools/install
# - Bun: https://bun.com/docs/installation
# - Tauri: https://tauri.app/start/prerequisites/
# - Protobuf: https://protobuf.dev/installation/

# 2. Clone & Build
git clone https://github.com/TM9657/flow-like.git
cd flow-like
bun install && bun run build:desktop
```

> 💡 **Platform-specific notes:** Check our [workflow automation documentation](https://docs.flow-like.com/) for hints on macOS, Windows, and Linux builds.

<br>

## 📸 Screenshots & Gallery

<details>
<summary><strong>🔒 Team & Access Management</strong></summary>
<p align="center"><img width="800" src="https://cdn.flow-like.com/website/TeamManagement.webp" alt="Manage Team Members"></p>
<p align="center"><img width="800" src="https://cdn.flow-like.com/website/RightsAndRoles.webp" alt="Set Rights and Roles"></p>
</details>

<details>
<summary><strong>🗄️ Built-in Storage & Search</strong></summary>
<p align="center"><img width="800" src="https://cdn.flow-like.com/website/Storage.webp" alt="Manage App Storage"></p>
<p align="center"><em>Files, tables, and hybrid search — right on the canvas. No extra services needed.</em></p>
</details>

<br>

## ❓ Frequently Asked Questions

<details>
<summary><strong>Is Flow-Like free to use?</strong></summary>

**Most likely, yes!** Flow-Like uses the Business Source License (BSL), which is free for the vast majority of use cases.

You can freely use, modify, and deploy Flow-Like if your organization has:
- Fewer than **2,000 employees**, and
- Less than **$300 million** in annual recurring revenue

This covers startups, SMBs, mid-market companies, and even many large enterprises. The source code is fully transparent and available for inspection and contribution. Organizations beyond these thresholds can contact us for commercial licensing options.

📄 [Read the full license terms](https://github.com/TM9657/flow-like/blob/main/LICENSE)

</details>

<details>
<summary><strong>Can I run Flow-Like completely offline?</strong></summary>

Absolutely. Flow-Like works 100% offline on your local machine. This makes it perfect for secure environments, air-gapped networks, or when you simply want to work without internet connectivity. You can switch to online mode anytime to collaborate with your team.

</details>

<details>
<summary><strong>How does Flow-Like compare to other workflow tools?</strong></summary>

Flow-Like is unique in offering **fully typed workflows** with complete data lineage tracking. Unlike traditional tools, you can see exactly where data came from, how it was transformed, and why results look the way they do. Plus, we're built on Rust for superior performance, and we're source available with no vendor lock-in.

</details>

<details>
<summary><strong>Can I embed Flow-Like in my own application?</strong></summary>

Yes! Flow-Like is white-label ready. You can embed the visual editor in your application, customize the theme to match your brand, integrate with your SSO, and even run just the engine behind the scenes. It's perfect for SaaS platforms and internal tools.

</details>

<details>
<summary><strong>What programming languages can I use with Flow-Like?</strong></summary>

Flow-Like has a visual no-code interface, so you don't need to code to create workflows. However, developers can create custom nodes using Rust, and we provide SDKs and APIs for programmatic control. The core engine is built in Rust for maximum performance.

</details>

<details>
<summary><strong>Is Flow-Like suitable for enterprise use?</strong></summary>

Absolutely. Flow-Like is **enterprise-ready from day one** with role-based access control (RBAC), complete audit trails, SSO integration, process compliance features, approval workflows, policy enforcement, air-gap deployment, and high-performance execution. Many organizations use Flow-Like for mission-critical automation.

</details>

<details>
<summary><strong>What about compliance and governance?</strong></summary>

Flow-Like includes built-in compliance features: complete audit trails for every workflow execution, role-based permissions, approval workflows, policy enforcement, and data lineage tracking. These features help you meet regulatory requirements like GDPR, SOC2, and industry-specific compliance standards.

</details>

<details>
<summary><strong>How do I get support?</strong></summary>

Join our [Discord community](https://discord.com/invite/mdBA9kMjFJ) for quick help, check the [documentation](https://docs.flow-like.com/), or open an issue on [GitHub](https://github.com/TM9657/flow-like/issues). We're here to help!

</details>

<details>
<summary><strong>Can I migrate from another workflow automation tool?</strong></summary>

While there's no automatic migration tool yet, our flexible node system and data import capabilities make it straightforward to rebuild workflows. Our community can help guide you through the process. Join our [Discord](https://discord.com/invite/mdBA9kMjFJ) for migration assistance.

</details>

<details>
<summary><strong>How stable is Flow-Like? Can I use it in production?</strong></summary>

Flow-Like is actively developed and used in production by many users. However, as with any automation platform, we recommend thorough testing before deploying mission-critical workflows. Check our [releases page](https://github.com/TM9657/flow-like/releases) for stability information on each version.

</details>

<br>

## 🤝 Join the Flow-Like Community

We'd love your help making Flow-Like the best open-source workflow automation platform!

<div align="center">

| 🐛 Report Issues | 💡 Discussions | 💬 Discord | 📦 Share Templates |
|:---:|:---:|:---:|:---:|
| [Create an Issue](https://github.com/TM9657/flow-like/issues) | [Join Discussions](https://github.com/TM9657/flow-like/discussions) | [Join Discord](https://discord.com/invite/mdBA9kMjFJ) | Share your flows as templates! |

</div>

**Ways to Contribute to Open-Source Workflow Automation:**
- 🐛 **Report Issues** — Found a bug? Request a feature via [GitHub Issues](https://github.com/TM9657/flow-like/issues)
- 💻 **Submit Code** — Fork the repo and create pull requests with improvements
- 💡 **Share Ideas** — Join our [community discussions](https://github.com/TM9657/flow-like/discussions) about workflow automation
- 📚 **Improve Docs** — Help others by writing tutorials and guides
- 🌟 **Spread the Word** — Star the repo and share Flow-Like with your network
- 🧩 **Build Integrations** — Create custom workflow nodes and share them
- 🎨 **Design Themes** — Contribute custom themes and UI improvements


<br>

## 📊 Project Stats & Analytics

<div align="center">

<table>
  <tr>
    <td align="center" width="50%">
      <img src="https://repobeats.axiom.co/api/embed/6fe5df31b9a96f584f8898beb4457bd8aa3852f1.svg" alt="Repobeats analytics" width="100%">
    </td>
    <td align="center" width="50%">
      <img src="https://api.star-history.com/svg?repos=TM9657/flow-like&type=Date" alt="Star History Chart" width="100%">
    </td>
  </tr>
</table>

</div>

<br>

## 🔗 Links

[🌐 Website](https://flow-like.com) • [📄 Documentation](https://docs.flow-like.com) • [📦 Download](https://flow-like.com/download) • [📝 Blog](https://flow-like.com/blog)

---

<p align="center">
  <strong>Made with ❤️ in Munich, Germany</strong><br>
  <sub>Automate • Explain • Trust</sub>
</p>

<p align="center">
  <a href="https://github.com/TM9657/flow-like/blob/main/LICENSE">License</a> •
  <a href="https://github.com/TM9657/flow-like/blob/main/CODE_OF_CONDUCT.md">Code of Conduct</a> •
  <a href="https://github.com/TM9657/flow-like/blob/main/SECURITY.md">Security</a>
</p>

</div>