# PHASE 3 指令卡：原子实现

> **目标**: 完成指定模块的代码实现，通过 CI 验证  
> **输入**: 任务包 (Task Package)  
> **输出**: 通过 CI 的代码 + API 摘要文档  
> **禁止**: 加载任何其他模块的实现细节

---

## 执行路径

```
D:\vcp\projects\Synch\Synch项目流程.md → 参考 Phase 3 章节
D:\vcp\projects\Synch\AGENT_QUICKSTART.md → 快速参考
```

---

## 任务包格式 (你将收到)

```yaml
task_id: "SYNCH-XXX"
module: "模块名"
agent_role: "实现工程师"

deliverables:
  - "src/modules/[模块]/index.ts"
  - "src/modules/[模块]/internal/service.ts"
  - "src/modules/[模块]/internal/repository.ts"

context_scope:  # 【严格限制】只能访问这些文件
  allowed_files:
    - "contracts/modules/[模块].contract.ts"
    - "contracts/types/*.ts"
  forbidden_patterns:
    - "src/modules/*/internal/**"      # 禁止看其他模块内部
    - "src/modules/![模块]/**"         # 禁止加载其他模块

test_strategy:
  local: "NONE"      # 本地不跑测试
  ci: "AUTO"         # Push 后自动触发
  quick_check: "npm run quick"
```

---

## 步骤清单

### Step 1: 环境准备 (2分钟)
```bash
cd D:\vcp\projects\Synch
npm install        # 仅安装类型检查工具
```

### Step 2: 编码实现 (主要工作)
- **严格遵循契约**: 只实现契约定义的接口
- **黑盒开发**: 不依赖任何未声明的模块
- **内部隔离**: 实现放 `internal/`，仅 `index.ts` 对外暴露

### Step 3: 本地快速检查 (必须)
```bash
npm run quick      # 类型检查 + 代码格式
```
- ❌ 若失败：修复至通过
- ✅ 若通过：进入下一步

### Step 4: 提交并推送
```bash
git checkout -b feature/[模块]-[task_id]
git add .
git commit -m "feat([模块]): [task_id] 实现 [功能简述]"
git push origin feature/[模块]-[task_id]
```

### Step 5: 查看 CI 结果
```bash
gh run watch       # 或打开 GitHub PR 页面查看
```

### Step 6: 生成 API 摘要
按模板生成：`docs/handoff/[task_id]-api-summary.md`

---

## 自检清单

- [ ] `npm run quick` 本地通过
- [ ] 已推送至远程分支
- [ ] GitHub Checks 全部绿色
- [ ] API 摘要文档已生成
- [ ] 未触碰任何 `forbidden_patterns` 中的文件

---

## 完成信号

创建 PR，标题格式：
> `[SYNCH-XXX] feat([模块]): 实现 [功能]`

PR 描述包含 API 摘要链接。

---

## 失败处理

| CI 失败类型 | 行动 |
|:---|:---|
| 类型错误 | 本地修复 → 重新 Push |
| 单元测试失败 | 查看 CI 日志 → 修复 → 重新 Push |
| 契约合规失败 | **立即停止**，联系主Agent (契约可能被误改) |
| 集成测试失败 | 可能依赖模块未就绪，等待或协调 |

---

## 紧急中止条件

- 发现契约定义存在逻辑矛盾
- CI 系统故障无法获取结果
- 需要访问 forbidden 文件才能完成任务