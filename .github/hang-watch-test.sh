#!/usr/bin/env bash
# hang-watch.sh 超时掐得干净（组织审计 #189）：命令留下一个后代拿着输出管道，过了 HANG_SECONDS 脚本要连它一起掐掉，读管道的
# 那一头随之结束，不用等那个后代自己完。改前：脚本 5 秒退出，管道照样等到后代 25 秒睡完。
set -u
here=$(cd "$(dirname "$0")" && pwd)
start=$(date +%s)
HANG_SECONDS=1 "$here/hang-watch.sh" bash -c 'sleep 25 & wait' 2>/dev/null | cat >/dev/null
status=${PIPESTATUS[0]}
took=$(( $(date +%s) - start ))
echo "hang-watch: exit $status after ${took}s"
if [ "$status" -ne 1 ]; then
  echo "::error::hang-watch 超时以后该判失败（退出码 1），实际 $status"
  exit 1
fi
if [ "$took" -ge 15 ]; then
  echo "::error::hang-watch 超时以后后代没掐掉，管道等了 ${took} 秒"
  exit 1
fi
# 没超时的照常跑完，退出码原样交回。
HANG_SECONDS=60 "$here/hang-watch.sh" bash -c 'exit 3'
passed=$?
if [ "$passed" -ne 3 ]; then
  echo "::error::hang-watch 没超时时该交回命令的退出码 3，实际 $passed"
  exit 1
fi
