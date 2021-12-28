# yevis-cli

**This repository is under-developing!!**

`ddbj/yevis-workflows` に workflow を登録するための cli tool

- PR として workflow を登録する
- 以下を local 環境で確認する
  - ワークフロー定義ファイルの文法が正しいこと
  - テスト用の入力データが用意されており、アクセス可能であること
  - テストデータを入力に、指定された実行系で実行可能であること
  - テスト実行時にエラーが検出されないこと

## Usage

```bash
yevis --make-template https://github.com/path/to/workflow-file
```

## Development

Run test.

```bash
cargo test
```
