use tikv_client::{
    TransactionClient,
    RawClient,
    Result,
    Config,
    Key,
    Value, 
    TransactionOptions, 
    Timestamp,
    Snapshot,
    ColumnFamily,
    Backoff,
    KvPair,
    BoundRange,
    proto::kvrpcpb::{Mutation, Op}
};

pub struct StateWriter {
    client: TransactionClient,
}

impl StateWriter {
    pub async fn new<'a>(
        pd_endpoints: Vec<&'a str>
    ) -> Result<Self> {
        let client = TransactionClient::new(pd_endpoints).await?;
        Ok(Self { client }) 
    }

    pub async fn new_with_config<'a>(
        pd_endpoints: Vec<&'a str>,
        config: Config
    ) -> Result<Self> {
        let client = TransactionClient::new_with_config(
            pd_endpoints,
            config
        ).await?;
        Ok(Self { client })
    }

    pub async fn put_optimistic(
        &self,
        k: impl Into<Key>,
        v: impl Into<Value>
    ) -> Result<()> {
        let mut txn = self.client.begin_optimistic().await?;
        txn.put(k, v).await?;
        txn.commit().await?;
        Ok(())
    }

    pub async fn put_pessimistic(
        &self,
        k: impl Into<Key>,
        v: impl Into<Value>
    ) -> Result<()> {
        let mut txn = self.client.begin_pessimistic().await?;
        txn.put(k, v).await?;
        txn.commit().await?;
        Ok(())
    }

    pub async fn put_with_options(
        &self,
        options: TransactionOptions,
        k: impl Into<Key> + Clone,
        v: impl Into<Value>
    ) -> Result<()> {
        let mut txn = self.client.begin_with_options(options).await?;
        txn.put(k, v).await?;
        txn.commit().await?;
        Ok(())
    }

    pub async fn batch_put(
        &self,
        kv_pairs: impl IntoIterator<Item = KvPair>
    ) -> Result<()> {
        let mut txn = self.client.begin_optimistic().await?;
        let mutations: Vec<Mutation> = kv_pairs.into_iter()
            .map(|kv| {
                let key: Key = kv.0.into();
                let value: Value = kv.1.into();
                Mutation {
                    op: Op::Put.into(),
                    key: key.into(),
                    value: value.into(), 
                    ..Default::default()
                }
            }).collect();

        txn.batch_mutate(mutations).await?;
        txn.commit().await?;
        Ok(())
    }

    pub fn snapshot(
        &self,
        timestamp: Timestamp,
        options: TransactionOptions
    ) -> Snapshot {
        self.client.snapshot(timestamp, options)
    }

    pub async fn current_timestamp(
        &self
    ) -> Result<Timestamp> {
        self.client.current_timestamp().await
    }

    pub async fn garbage_collect(
        &self,
        safepoint: Timestamp
    ) -> Result<bool> {
        self.client.gc(safepoint).await
    }
}

pub struct StateReader {
    client: RawClient
}

impl StateReader {
    pub async fn new<'a>(
        pd_endpoints: Vec<&'a str>
    ) -> Result<Self> {
        let client = RawClient::new(pd_endpoints).await?;
        Ok(Self { client })
    }

    pub async fn new_with_config<'a>(
        pd_endpoints: Vec<&'a str>,
        config: Config
    ) -> Result<Self> {
        let client = RawClient::new_with_config(
            pd_endpoints,
            config
        ).await?;
        Ok(Self { client })
    }

    pub fn new_with_column_family(
        self,
        cf: ColumnFamily
    ) -> Self {
        let client = self.client.with_cf(cf);
        Self { client }
    }

    pub fn with_backoff(
        self,
        backoff: Backoff
    ) -> Self {
        let client = self.client.with_backoff(backoff);
        Self { client }
    }

    pub async fn get(
        &self,
        key: impl Into<Key>
    ) -> Result<Option<Value>> {
        self.client.get(key).await
    }

    pub async fn batch_get(
        &self,
        keys: impl IntoIterator<Item = impl Into<Key>>
    ) -> Result<Vec<KvPair>> {
        self.client.batch_get(keys).await
    }

    pub async fn batch_scan(
        &self,
        ranges: impl IntoIterator<Item = impl Into<BoundRange>>, 
        each_limit: u32
    ) -> Result<Vec<KvPair>> {
        self.client.batch_scan(
            ranges,
            each_limit
        ).await
    }

    pub async fn batch_scan_keys(
        &self,
        ranges: impl IntoIterator<Item = impl Into<BoundRange>>,
        each_limit: u32
    ) -> Result<Vec<Key>> {
        self.client.batch_scan_keys(
            ranges,
            each_limit
        ).await
    }

    pub async fn scan(
        &self,
        range: impl Into<BoundRange>,
        limit: u32
    ) -> Result<Vec<KvPair>> {
        self.client.scan(
            range,
            limit
        ).await
    }

    pub async fn scan_keys(
        &self,
        range: impl Into<BoundRange>,
        limit: u32
    ) -> Result<Vec<Key>> {
        self.client.scan_keys(
            range,
            limit
        ).await
    }
}
