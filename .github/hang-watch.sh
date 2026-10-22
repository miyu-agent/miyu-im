#!/usr/bin/env bash
# 门禁卡住时留下证据（I-24：macOS 上集成测试偶尔卡到 45 分钟被掐，日志里什么都没有）：照常跑参数里的命令，
# 过了 HANG_SECONDS 还没完的，用 sample（macOS）抓每个测试进程的栈印出来，再掐掉、判失败。
set -u
"$@" &
PID=$!
LIMIT=${HANG_SECONDS:-1800}
for ((waited = 0; waited < LIMIT; waited += 5)); do
  kill -0 "$PID" 2>/dev/null || break
  sleep 5
done
if kill -0 "$PID" 2>/dev/null; then
  echo "::error::门禁过了 ${LIMIT} 秒还没完，抓栈"
  # 只认程序本身在 target/debug/deps 下的（测试程序），参数里带这个路径的编译进程不算。
  TESTS=$(ps -axww -o pid=,command= | awk '$2 ~ /target\/debug\/deps\// {print $1}')
  for T in $TESTS; do ps -ww -o pid=,etime=,command= -p "$T"; done
  for T in $TESTS; do
    echo "===== sample $T"
    sample "$T" 5 -file "${RUNNER_TEMP:-/tmp}/sample-$T.txt" >/dev/null 2>&1 && cat "${RUNNER_TEMP:-/tmp}/sample-$T.txt"
  done
  kill -9 "$PID"
  exit 1
fi
wait "$PID"
