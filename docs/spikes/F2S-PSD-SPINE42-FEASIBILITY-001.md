# F2S-PSD-SPINE42-FEASIBILITY-001

## PSD裁决

项目采用自有最小PSD bytes探针验证两层RGBA、层名、尺寸、alpha和稳定SHA-256。探针不依赖未审计运行时包；产品级PSD Adapter仍需后续对大图内存、更多层、Photoshop与Spine Editor UI导入执行独立fixture测试。PSD写出属于开放candidate，不能替代人工分层审批。

## Spine 4.2.43静态裁决

`F2S-SPINE-CAP-4.2.43-001`由M00-005唯一拥有。开放fixture固定`skeleton.spine=4.2.43`、Rig IR i64 tick和`timeBase=1/30000`；仅Adapter把tick转换为秒。静态校验拒绝错误patch、非有限数、路径穿越和缺attachment。

## 用户工具往返

`spine42-evidence-check.mjs`只校验用户提供的显式lease和已有输出证据，不启动未批准工具，也不写`.atlas/.spine/.skel`。lease必须包含4.2.43、许可确认、有效期、允许输出根、writer provenance和逐文件SHA-256。没有合法工具时状态为`NOT_RUN/EXTERNAL`，不能以静态测试冒充Editor兼容VERIFIED。

## 当前状态

- 最小PSD静态探针：待本机执行。
- Spine开放fixture静态契约：待生成器执行。
- Editor/CLI往返：`EXTERNAL`，需要用户合法4.2.43工具。
- 发布授权：false。
