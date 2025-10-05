# ツール作成目的
勉強

# 用途
自分用の対話型専用のシェル

# 環境変数
* MY_SHELL_RC
* MY_SHELL_HISTORY

# 設計メモ
* 全体をinput, output, shellの3つに分けて考える。
* inputはどのkeyを押下したかの判別までを担当する。
* outputは画面出力のすべてを担当する。
* inputをshellで処理し、shellからoutputを呼び出す。

# 制限事項
1. 