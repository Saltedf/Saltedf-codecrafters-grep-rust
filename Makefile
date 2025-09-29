# 使用 .PHONY 来声明 submittest 是一个“伪目标”，而不是一个真实的文件名。
# 这样做可以防止因为目录下恰好有一个叫“submittest”的文件而导致命令失效。
.PHONY: submittest

# 定义变量，?= 表示如果变量未定义，则使用此默认值。
# 这允许我们从命令行覆盖它。
MSG ?= "Test: Work in Progress"
BRANCH ?= "master"

# 定义我们的 acp 任务
submittest:
	# @符号会阻止 make 打印出这条命令本身，使输出更干净。
	# 警告：Makefile中的缩进必须是Tab键，而不是空格！这是它最古怪的地方。
	@echo ">>> 1. Staging all changes..."
	@cargo check
	@echo ">>> 2. Committing with message: \"$(MSG)\""
	@git commit --allow-empty -am "submit & test"
	@echo ">>> 3. Pushing to origin/$(BRANCH)..."
	@git push origin $(BRANCH)
	@echo "✅ Done."