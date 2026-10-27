# XAUUSD tick data

Place XAUUSD Parquet tick files in this directory or in year subdirectories.
Each Parquet file must contain numeric `ts` (Unix seconds), `price`, and
`volume` columns. The application reads nested `*.parquet` files from this
directory.
