# ツール作成目的
勉強

# 用途
自分用の対話型専用のシェル

# 環境変数
* MY_SHELL_RC
* MY_SHELL_HISTORY
* MY_SHELL_HISTORY_CAPACITY
* MY_SHELL_COMPLETION

# 制限事項
1. 対話型で普段使わないので、"関数定義、for、if、&、fg、bg、^Z"を実装していない
2. aliasのネストを無効化
3. sourceコマンドや.rcファイルにおいて、aliasとabbrの展開を実施しない

# メモ
* 残りは、変数展開とタブ補完 
