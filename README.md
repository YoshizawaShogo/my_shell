# ツール作成目的
勉強

# 用途
自分用の対話型専用のシェル

# 環境変数(初期値)
* MY_SHELL_RC($HOME/.my_shell_rc)
* MY_SHELL_HISTORY($HOME/.my_shell_history)
* MY_SHELL_HISTORY_CAPACITY(1000)
* MY_SHELL_COMPLETION($HOME/.my_shell_completion)

# 制限事項
1. 対話型で使わないので、"関数定義、for、if、&、fg、bg、^Z"を実装していない
2. aliasのネストを無効化
3. sourceコマンドや.rcファイルにおいて、aliasとabbrの展開を実施しない
4. 変数の補完

# メモ
* 
