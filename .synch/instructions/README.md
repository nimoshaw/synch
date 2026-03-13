# Synch Agent 指令中心

> 给 Agent 的极简入口：知道路径，选对 Phase，执行即可。

---

## 快速开始

**你是哪个 Phase 的 Agent？**

| 阶段 | 身份 | 执行文件 |
|:---|:---|:---|
| Phase 1 | 架构师 | `PHASE_1.md` |
| Phase 2 | 规范工程师 | `PHASE_2.md` |
| Phase 3 | 实现工程师 | `PHASE_3.md` |
| Phase 4 | 集成架构师 | `PHASE_4.md` |

---

## 全局参考

- 完整流程文档：`../Synch项目流程.md`
- 快速命令参考：`../AGENT_QUICKSTART.md`

---

## 任务包模板 (Phase 3 用)

复制并填写：

```yaml
task_id: "SYNCH-XXX"
module: "[模块名]"
agent_role: "实现工程师"

deliverables:
  - "src/modules/[模块]/index.ts"
  - "src/modules/[模块]/internal/service.ts"
  - "src/modules/[模块]/internal/repository.ts"

context_scope:
  allowed_files:
    - "contracts/modules/[模块].contract.ts"
    - "contracts/types/*.ts"
  forbidden_patterns:
    - "src/modules/*/internal/**"
    - "src/modules/![模块]/**"

test_strategy:
  local: "NONE"
  ci: "AUTO"
  quick_check: "npm run quick"
```