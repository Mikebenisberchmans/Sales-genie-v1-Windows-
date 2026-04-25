import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";

const DEFAULT_CFG = {
  salesperson: { name: "", id: "" },
  objectStore: {
    provider: "none",
    s3:    { bucket: "", region: "us-east-1", accessKeyId: "", secretAccessKey: "", prefix: "" },
    azure: { accountName: "", accountKey: "", containerName: "" },
    gcs:   { bucket: "", serviceAccountKey: "" },
  },
  warehouse: {
    provider: "none",
    snowflake:  { account: "", username: "", password: "", database: "", schema: "", warehouse: "", table: "genie_recordings" },
    bigquery:   { projectId: "", datasetId: "", tableId: "genie_recordings", serviceAccountKey: "" },
    clickhouse: { host: "localhost", port: 8123, database: "default", table: "genie_recordings", username: "default", password: "" },
    databricks: { host: "", httpPath: "", accessToken: "", catalog: "main", schema: "default", table: "genie_recordings" },
    redshift:   { host: "", region: "us-east-1", port: 5439, database: "", schema: "public", table: "genie_recordings", username: "", accessKeyId: "", secretAccessKey: "" },
  },
  analysis: {
    enabled: false,
    inferenceEndpoint: "",
    whisperModel: "base",
    sttPort: 8765,
  },
};

function deepMerge(base, override) {
  if (!override) return base;
  const result = { ...base };
  for (const key of Object.keys(override)) {
    if (override[key] !== null && typeof override[key] === "object" && !Array.isArray(override[key])) {
      result[key] = deepMerge(base[key] ?? {}, override[key]);
    } else {
      result[key] = override[key];
    }
  }
  return result;
}

/* ── Tab icons ─────────────────────────────────────────────────────── */
const PersonIcon = ({ size = 20 }) => (
  <svg viewBox="0 0 24 24" width={size} height={size} fill="none">
    <circle cx="12" cy="8" r="4" stroke="currentColor" strokeWidth="1.8" fill="none"/>
    <path d="M4 20c0-4 3.6-7 8-7s8 3 8 7" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" fill="none"/>
  </svg>
);

const BucketIcon = ({ size = 20 }) => (
  <svg viewBox="0 0 24 24" width={size} height={size} fill="none">
    <path d="M7 6C7 4.34 9.24 3 12 3s5 1.34 5 3" stroke="currentColor" strokeWidth="1.8" fill="none"/>
    <path d="M4.5 8.5C4.5 6.57 7.91 5 12 5s7.5 1.57 7.5 3.5L18 19c0 1.1-2.69 2-6 2s-6-.9-6-2L4.5 8.5z" stroke="currentColor" strokeWidth="1.8" fill="none"/>
    <ellipse cx="12" cy="8.5" rx="7.5" ry="2" stroke="currentColor" strokeWidth="1.8" fill="none"/>
  </svg>
);

const DatabaseIcon = ({ size = 20 }) => (
  <svg viewBox="0 0 24 24" width={size} height={size} fill="none">
    <ellipse cx="12" cy="6" rx="8" ry="3" stroke="currentColor" strokeWidth="1.8" fill="none"/>
    <path d="M4 6v4c0 1.66 3.58 3 8 3s8-1.34 8-3V6" stroke="currentColor" strokeWidth="1.8" fill="none"/>
    <path d="M4 10v4c0 1.66 3.58 3 8 3s8-1.34 8-3v-4" stroke="currentColor" strokeWidth="1.8" fill="none"/>
    <path d="M4 14v4c0 1.66 3.58 3 8 3s8-1.34 8-3v-4" stroke="currentColor" strokeWidth="1.8" fill="none"/>
  </svg>
);

const SparkleIcon = ({ size = 20 }) => (
  <svg viewBox="0 0 24 24" width={size} height={size} fill="none">
    <path d="M12 2l2.4 7.2H22l-6.2 4.5 2.4 7.3L12 17l-6.2 4-0.0 0 2.4-7.3L2 9.2h7.6z" stroke="currentColor" strokeWidth="1.6" strokeLinejoin="round" fill="none"/>
    <circle cx="19" cy="4" r="1.5" fill="currentColor" opacity="0.6"/>
    <circle cx="5" cy="18" r="1" fill="currentColor" opacity="0.5"/>
  </svg>
);

/* ── Brand icons ───────────────────────────────────────────────────── */
const S3Icon = () => (
  <svg viewBox="0 0 40 40" width="22" height="22">
    <rect width="40" height="40" rx="8" fill="#FF9900"/>
    <path d="M20 8l8 4v16l-8 4-8-4V12l8-4z" fill="none" stroke="white" strokeWidth="1.5"/>
    <path d="M12 12l8 4 8-4M20 16v12" stroke="white" strokeWidth="1.5" fill="none"/>
  </svg>
);
const AzureIcon = () => (
  <svg viewBox="0 0 40 40" width="22" height="22">
    <rect width="40" height="40" rx="8" fill="#0078D4"/>
    <path d="M14 10h7l-3 11 7 9H10l4-20z" fill="white"/>
    <path d="M21 10h5l-9 20h-3l7-20z" fill="rgba(255,255,255,0.6)"/>
  </svg>
);
const GCSIcon = () => (
  <svg viewBox="0 0 40 40" width="22" height="22">
    <rect width="40" height="40" rx="8" fill="#4285F4"/>
    <rect x="10" y="17" width="20" height="6" rx="3" fill="white"/>
    <circle cx="20" cy="14" r="5" fill="white" opacity="0.9"/>
    <circle cx="13" cy="26" r="4" fill="#FBBC04"/>
    <circle cx="27" cy="26" r="4" fill="#34A853"/>
  </svg>
);
const SnowflakeIcon = () => (
  <svg viewBox="0 0 40 40" width="22" height="22">
    <rect width="40" height="40" rx="8" fill="#29B5E8"/>
    <line x1="20" y1="8" x2="20" y2="32" stroke="white" strokeWidth="2.5" strokeLinecap="round"/>
    <line x1="8" y1="20" x2="32" y2="20" stroke="white" strokeWidth="2.5" strokeLinecap="round"/>
    <line x1="11.7" y1="11.7" x2="28.3" y2="28.3" stroke="white" strokeWidth="2.5" strokeLinecap="round"/>
    <line x1="28.3" y1="11.7" x2="11.7" y2="28.3" stroke="white" strokeWidth="2.5" strokeLinecap="round"/>
    <circle cx="20" cy="20" r="3" fill="white"/>
  </svg>
);
const BigQueryIcon = () => (
  <svg viewBox="0 0 40 40" width="22" height="22">
    <rect width="40" height="40" rx="8" fill="#4285F4"/>
    <circle cx="18" cy="18" r="7" fill="none" stroke="white" strokeWidth="2.5"/>
    <line x1="23" y1="23" x2="31" y2="31" stroke="#FBBC04" strokeWidth="3" strokeLinecap="round"/>
  </svg>
);
const ClickHouseIcon = () => (
  <svg viewBox="0 0 40 40" width="22" height="22">
    <rect width="40" height="40" rx="8" fill="#FACC14"/>
    <rect x="10" y="12" width="4" height="16" rx="2" fill="#1a1a1a"/>
    <rect x="16" y="10" width="4" height="20" rx="2" fill="#1a1a1a"/>
    <rect x="22" y="14" width="4" height="12" rx="2" fill="#1a1a1a"/>
    <rect x="28" y="16" width="4" height="8"  rx="2" fill="#1a1a1a" opacity="0.5"/>
  </svg>
);
const DatabricksIcon = () => (
  <svg viewBox="0 0 40 40" width="22" height="22">
    <rect width="40" height="40" rx="8" fill="#FF3621"/>
    <path d="M20 9l11 6.5v6L20 16 9 22.5v-6L20 9z" fill="white"/>
    <path d="M9 22.5l11 6.5 11-6.5v6L20 35 9 28.5v-6z" fill="rgba(255,255,255,0.65)"/>
  </svg>
);
const RedshiftIcon = () => (
  <svg viewBox="0 0 40 40" width="22" height="22">
    <rect width="40" height="40" rx="8" fill="#8C4FFF"/>
    <ellipse cx="20" cy="15" rx="10" ry="5" fill="white" opacity="0.9"/>
    <path d="M10 15v10c0 2.8 4.5 5 10 5s10-2.2 10-5V15" fill="none" stroke="white" strokeWidth="2" opacity="0.7"/>
    <path d="M10 20c0 2.8 4.5 5 10 5s10-2.2 10-5" fill="none" stroke="white" strokeWidth="1.5" opacity="0.5"/>
  </svg>
);
const NoneIcon = () => (
  <svg viewBox="0 0 40 40" width="18" height="18">
    <circle cx="20" cy="20" r="10" fill="none" stroke="#475569" strokeWidth="2" strokeDasharray="4 3"/>
    <line x1="13" y1="13" x2="27" y2="27" stroke="#475569" strokeWidth="2"/>
  </svg>
);

const OS_PROVIDERS = [
  { id:"s3",    name:"Amazon S3",    Icon: S3Icon },
  { id:"azure", name:"Azure Blob",   Icon: AzureIcon },
  { id:"gcs",   name:"Google Cloud", Icon: GCSIcon },
];
const WH_PROVIDERS = [
  { id:"snowflake",  name:"Snowflake",  Icon: SnowflakeIcon },
  { id:"bigquery",   name:"BigQuery",   Icon: BigQueryIcon },
  { id:"clickhouse", name:"ClickHouse", Icon: ClickHouseIcon },
  { id:"databricks", name:"Databricks", Icon: DatabricksIcon },
  { id:"redshift",   name:"Redshift",   Icon: RedshiftIcon },
];

/* ── Reusable components ───────────────────────────────────────────── */
function Field({ label, hint, children }) {
  return (
    <div className="field">
      <label>{label}</label>
      {children}
      {hint && <div className="field-hint">{hint}</div>}
    </div>
  );
}

function ProviderGrid({ providers, selected, onSelect }) {
  return (
    <div>
      <div className="provider-grid">
        {providers.map(({ id, name, Icon }) => (
          <div
            key={id}
            className={`provider-card ${selected === id ? "selected" : ""}`}
            onClick={() => onSelect(id)}
          >
            <div className="p-icon"><Icon /></div>
            <div className="p-name">{name}</div>
          </div>
        ))}
        <div
          className={`p-none-card ${selected === "none" ? "selected" : ""}`}
          onClick={() => onSelect("none")}
        >
          <NoneIcon />
          <span>None</span>
        </div>
      </div>
    </div>
  );
}

/* ── Main component ────────────────────────────────────────────────── */
export default function ConfigLedger() {
  const [cfg, setCfg]       = useState(DEFAULT_CFG);
  const [activeTab, setActiveTab] = useState(0);
  const [status, setStatus] = useState(null);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    invoke("get_config").then((saved) => {
      if (saved) setCfg(deepMerge(DEFAULT_CFG, saved));
    });
  }, []);

  const set = (path, value) => {
    setCfg((prev) => {
      const next = structuredClone(prev);
      const keys = path.split(".");
      let obj = next;
      for (let i = 0; i < keys.length - 1; i++) obj = obj[keys[i]];
      obj[keys[keys.length - 1]] = value;
      return next;
    });
  };

  const handleSave = async () => {
    setSaving(true);
    try {
      await invoke("save_config", { config: cfg });
      setStatus({ type: "success", msg: "Configuration saved successfully." });
      setTimeout(() => getCurrentWindow().hide(), 1200);
    } catch (e) {
      setStatus({ type: "error", msg: `Save failed: ${e}` });
    } finally {
      setSaving(false);
    }
  };

  const handleClose = () => getCurrentWindow().hide();

  const osProvider = cfg.objectStore.provider;
  const whProvider = cfg.warehouse.provider;
  const isSalespersonSet = !!(cfg.salesperson.name && cfg.salesperson.id);
  const isStoreSet  = osProvider !== "none";
  const isWareSet   = whProvider !== "none";
  const initials    = cfg.salesperson.name
    ? cfg.salesperson.name.split(" ").map(w => w[0]).join("").slice(0, 2).toUpperCase()
    : "?";

  const TABS = [
    { label: "Salesperson",  Icon: PersonIcon,   configured: isSalespersonSet },
    { label: "Object Store", Icon: BucketIcon,   configured: isStoreSet },
    { label: "Warehouse",    Icon: DatabaseIcon,  configured: isWareSet },
    { label: "AI Analysis",  Icon: SparkleIcon,   configured: cfg.analysis.enabled },
  ];

  return (
    <div className="ledger">
      {/* Header */}
      <div className="ledger-header">
        <div className="ledger-header-icon">
          <svg viewBox="0 0 24 24" width="18" height="18">
            <path d="M4 6h16M4 10h16M4 14h16M4 18h10" stroke="white" strokeWidth="2.2" strokeLinecap="round" fill="none"/>
          </svg>
        </div>
        <div>
          <h1>Genie Configuration</h1>
          <p>Connect cloud storage, warehouse, and AI analysis</p>
        </div>
      </div>

      {/* Horizontal Tab Bar */}
      <div className="tab-bar">
        {TABS.map(({ label, Icon, configured }, i) => (
          <button
            key={i}
            className={`tab-btn ${activeTab === i ? "active" : ""}`}
            onClick={() => setActiveTab(i)}
          >
            {configured && <span className="tab-dot" />}
            <div className="tab-icon-wrap">
              <Icon size={18} />
            </div>
            <span className="tab-label">{label}</span>
          </button>
        ))}
      </div>

      <div className="ledger-body">
        {status && (
          <div className={`status-banner ${status.type}`}>
            <span>{status.type === "success" ? "✓" : "✗"}</span>
            {status.msg}
          </div>
        )}

        {/* ── Tab 0: Salesperson ───────────────────────── */}
        {activeTab === 0 && (
          <div className="tab-panel">
            {isSalespersonSet && (
              <div className="avatar-row">
                <div className="avatar-circle">{initials}</div>
                <div>
                  <div className="avatar-name">{cfg.salesperson.name}</div>
                  <div className="avatar-meta">{cfg.salesperson.id}</div>
                </div>
              </div>
            )}
            <div className="field-row">
              <Field label="Full Name">
                <input
                  value={cfg.salesperson.name}
                  onChange={e => set("salesperson.name", e.target.value)}
                  placeholder="Jane Smith"
                />
              </Field>
              <Field label="Employee ID">
                <input
                  value={cfg.salesperson.id}
                  onChange={e => set("salesperson.id", e.target.value)}
                  placeholder="EMP-001"
                />
              </Field>
            </div>
          </div>
        )}

        {/* ── Tab 1: Object Store ──────────────────────── */}
        {activeTab === 1 && (
          <div className="tab-panel">
            <ProviderGrid
              providers={OS_PROVIDERS}
              selected={osProvider}
              onSelect={id => set("objectStore.provider", id)}
            />

            {osProvider === "s3" && (
              <div className="provider-fields">
                <div className="field-row">
                  <Field label="Bucket">
                    <input value={cfg.objectStore.s3.bucket} onChange={e => set("objectStore.s3.bucket", e.target.value)} placeholder="my-recordings-bucket" />
                  </Field>
                  <Field label="Region">
                    <input value={cfg.objectStore.s3.region} onChange={e => set("objectStore.s3.region", e.target.value)} placeholder="us-east-1" />
                  </Field>
                </div>
                <Field label="Access Key ID">
                  <input value={cfg.objectStore.s3.accessKeyId} onChange={e => set("objectStore.s3.accessKeyId", e.target.value)} placeholder="AKIAIOSFODNN7EXAMPLE" />
                </Field>
                <Field label="Secret Access Key">
                  <input type="password" value={cfg.objectStore.s3.secretAccessKey} onChange={e => set("objectStore.s3.secretAccessKey", e.target.value)} placeholder="••••••••••••••••••••••••••••••••••••••••" />
                </Field>
                <Field label="Prefix" hint="Files stored as: {prefix}/{opp_id}/mic_timestamp.wav">
                  <input value={cfg.objectStore.s3.prefix} onChange={e => set("objectStore.s3.prefix", e.target.value)} placeholder="calls/recordings (optional)" />
                </Field>
              </div>
            )}
            {osProvider === "azure" && (
              <div className="provider-fields">
                <div className="field-row">
                  <Field label="Account Name">
                    <input value={cfg.objectStore.azure.accountName} onChange={e => set("objectStore.azure.accountName", e.target.value)} placeholder="mystorageaccount" />
                  </Field>
                  <Field label="Container">
                    <input value={cfg.objectStore.azure.containerName} onChange={e => set("objectStore.azure.containerName", e.target.value)} placeholder="recordings" />
                  </Field>
                </div>
                <Field label="Account Key">
                  <input type="password" value={cfg.objectStore.azure.accountKey} onChange={e => set("objectStore.azure.accountKey", e.target.value)} placeholder="Base64-encoded storage account key" />
                </Field>
              </div>
            )}
            {osProvider === "gcs" && (
              <div className="provider-fields">
                <Field label="Bucket">
                  <input value={cfg.objectStore.gcs.bucket} onChange={e => set("objectStore.gcs.bucket", e.target.value)} placeholder="my-recordings-bucket" />
                </Field>
                <Field label="Service Account Key (JSON)" hint="Paste the full contents of your service account JSON key file">
                  <textarea value={cfg.objectStore.gcs.serviceAccountKey} onChange={e => set("objectStore.gcs.serviceAccountKey", e.target.value)} placeholder={'{\n  "type": "service_account",\n  "project_id": "...",\n  ...\n}'} rows={5} />
                </Field>
              </div>
            )}
          </div>
        )}

        {/* ── Tab 2: Data Warehouse ────────────────────── */}
        {activeTab === 2 && (
          <div className="tab-panel">
            <ProviderGrid
              providers={WH_PROVIDERS}
              selected={whProvider}
              onSelect={id => set("warehouse.provider", id)}
            />

            {whProvider === "snowflake" && (
              <div className="provider-fields">
                <Field label="Account Identifier" hint="e.g. xy12345.us-east-1">
                  <input value={cfg.warehouse.snowflake.account} onChange={e => set("warehouse.snowflake.account", e.target.value)} placeholder="xy12345.us-east-1" />
                </Field>
                <div className="field-row">
                  <Field label="Username">
                    <input value={cfg.warehouse.snowflake.username} onChange={e => set("warehouse.snowflake.username", e.target.value)} placeholder="MY_USER" />
                  </Field>
                  <Field label="Password">
                    <input type="password" value={cfg.warehouse.snowflake.password} onChange={e => set("warehouse.snowflake.password", e.target.value)} placeholder="••••••••" />
                  </Field>
                </div>
                <div className="field-row">
                  <Field label="Database">
                    <input value={cfg.warehouse.snowflake.database} onChange={e => set("warehouse.snowflake.database", e.target.value)} placeholder="SALES_DB" />
                  </Field>
                  <Field label="Schema">
                    <input value={cfg.warehouse.snowflake.schema} onChange={e => set("warehouse.snowflake.schema", e.target.value)} placeholder="PUBLIC" />
                  </Field>
                </div>
                <div className="field-row">
                  <Field label="Warehouse">
                    <input value={cfg.warehouse.snowflake.warehouse} onChange={e => set("warehouse.snowflake.warehouse", e.target.value)} placeholder="COMPUTE_WH" />
                  </Field>
                  <Field label="Table">
                    <input value={cfg.warehouse.snowflake.table} onChange={e => set("warehouse.snowflake.table", e.target.value)} placeholder="genie_recordings" />
                  </Field>
                </div>
              </div>
            )}
            {whProvider === "bigquery" && (
              <div className="provider-fields">
                <div className="field-row">
                  <Field label="Project ID">
                    <input value={cfg.warehouse.bigquery.projectId} onChange={e => set("warehouse.bigquery.projectId", e.target.value)} placeholder="my-gcp-project" />
                  </Field>
                  <Field label="Dataset ID">
                    <input value={cfg.warehouse.bigquery.datasetId} onChange={e => set("warehouse.bigquery.datasetId", e.target.value)} placeholder="sales_data" />
                  </Field>
                </div>
                <Field label="Table ID">
                  <input value={cfg.warehouse.bigquery.tableId} onChange={e => set("warehouse.bigquery.tableId", e.target.value)} placeholder="genie_recordings" />
                </Field>
                <Field label="Service Account Key (JSON)" hint="Paste the full service account JSON">
                  <textarea value={cfg.warehouse.bigquery.serviceAccountKey} onChange={e => set("warehouse.bigquery.serviceAccountKey", e.target.value)} placeholder={'{\n  "type": "service_account",\n  ...\n}'} rows={5} />
                </Field>
              </div>
            )}
            {whProvider === "clickhouse" && (
              <div className="provider-fields">
                <div className="field-row">
                  <Field label="Host">
                    <input value={cfg.warehouse.clickhouse.host} onChange={e => set("warehouse.clickhouse.host", e.target.value)} placeholder="localhost" />
                  </Field>
                  <Field label="Port">
                    <input type="number" value={cfg.warehouse.clickhouse.port} onChange={e => set("warehouse.clickhouse.port", Number(e.target.value))} placeholder="8123" />
                  </Field>
                </div>
                <div className="field-row">
                  <Field label="Database">
                    <input value={cfg.warehouse.clickhouse.database} onChange={e => set("warehouse.clickhouse.database", e.target.value)} placeholder="default" />
                  </Field>
                  <Field label="Table">
                    <input value={cfg.warehouse.clickhouse.table} onChange={e => set("warehouse.clickhouse.table", e.target.value)} placeholder="genie_recordings" />
                  </Field>
                </div>
                <div className="field-row">
                  <Field label="Username">
                    <input value={cfg.warehouse.clickhouse.username} onChange={e => set("warehouse.clickhouse.username", e.target.value)} placeholder="default" />
                  </Field>
                  <Field label="Password">
                    <input type="password" value={cfg.warehouse.clickhouse.password} onChange={e => set("warehouse.clickhouse.password", e.target.value)} placeholder="••••••••" />
                  </Field>
                </div>
              </div>
            )}
            {whProvider === "databricks" && (
              <div className="provider-fields">
                <Field label="Workspace Host">
                  <input value={cfg.warehouse.databricks.host} onChange={e => set("warehouse.databricks.host", e.target.value)} placeholder="adb-1234567890.azuredatabricks.net" />
                </Field>
                <Field label="HTTP Path" hint="SQL Warehouse → Connection Details">
                  <input value={cfg.warehouse.databricks.httpPath} onChange={e => set("warehouse.databricks.httpPath", e.target.value)} placeholder="/sql/1.0/warehouses/abc123" />
                </Field>
                <Field label="Access Token">
                  <input type="password" value={cfg.warehouse.databricks.accessToken} onChange={e => set("warehouse.databricks.accessToken", e.target.value)} placeholder="dapi••••••••••••••••••••••••••••••••" />
                </Field>
                <div className="field-row">
                  <Field label="Catalog">
                    <input value={cfg.warehouse.databricks.catalog} onChange={e => set("warehouse.databricks.catalog", e.target.value)} placeholder="main" />
                  </Field>
                  <Field label="Schema">
                    <input value={cfg.warehouse.databricks.schema} onChange={e => set("warehouse.databricks.schema", e.target.value)} placeholder="default" />
                  </Field>
                </div>
                <Field label="Table">
                  <input value={cfg.warehouse.databricks.table} onChange={e => set("warehouse.databricks.table", e.target.value)} placeholder="genie_recordings" />
                </Field>
              </div>
            )}
            {whProvider === "redshift" && (
              <div className="provider-fields">
                <Field label="Cluster Identifier" hint="The cluster name, not the full endpoint">
                  <input value={cfg.warehouse.redshift.host} onChange={e => set("warehouse.redshift.host", e.target.value)} placeholder="my-cluster" />
                </Field>
                <div className="field-row">
                  <Field label="Region">
                    <input value={cfg.warehouse.redshift.region} onChange={e => set("warehouse.redshift.region", e.target.value)} placeholder="us-east-1" />
                  </Field>
                  <Field label="Database">
                    <input value={cfg.warehouse.redshift.database} onChange={e => set("warehouse.redshift.database", e.target.value)} placeholder="dev" />
                  </Field>
                </div>
                <div className="field-row">
                  <Field label="Schema">
                    <input value={cfg.warehouse.redshift.schema} onChange={e => set("warehouse.redshift.schema", e.target.value)} placeholder="public" />
                  </Field>
                  <Field label="Table">
                    <input value={cfg.warehouse.redshift.table} onChange={e => set("warehouse.redshift.table", e.target.value)} placeholder="genie_recordings" />
                  </Field>
                </div>
                <Field label="DB Username">
                  <input value={cfg.warehouse.redshift.username} onChange={e => set("warehouse.redshift.username", e.target.value)} placeholder="awsuser" />
                </Field>
                <Field label="AWS Access Key ID">
                  <input value={cfg.warehouse.redshift.accessKeyId} onChange={e => set("warehouse.redshift.accessKeyId", e.target.value)} placeholder="AKIAIOSFODNN7EXAMPLE" />
                </Field>
                <Field label="AWS Secret Access Key">
                  <input type="password" value={cfg.warehouse.redshift.secretAccessKey} onChange={e => set("warehouse.redshift.secretAccessKey", e.target.value)} placeholder="••••••••••••••••••••••••••••••••••••••••" />
                </Field>
              </div>
            )}
          </div>
        )}

        {/* ── Tab 3: AI Analysis ───────────────────────── */}
        {activeTab === 3 && (
          <div className="tab-panel">
            <div className="toggle-row">
              <div>
                <div className="toggle-label">AI Transcription & Analysis</div>
                <div className="toggle-desc">Whisper STT (bundled) + Phi-4 on Modal cloud</div>
              </div>
              <button
                className={`toggle-switch ${cfg.analysis.enabled ? "on" : ""}`}
                onClick={() => set("analysis.enabled", !cfg.analysis.enabled)}
              />
            </div>

            {/* First-time setup callout */}
            {cfg.analysis.enabled && !cfg.analysis.inferenceEndpoint && (
              <div style={{
                background: "rgba(16,185,129,0.08)",
                border: "1px solid rgba(16,185,129,0.2)",
                borderRadius: 12,
                padding: "14px 16px",
                marginBottom: 12,
              }}>
                <div style={{ fontSize: 12, fontWeight: 700, color: "#34d399", marginBottom: 6 }}>
                  🚀 First-time setup
                </div>
                <div style={{ fontSize: 11, color: "#64748b", lineHeight: 1.6 }}>
                  Run <span style={{ fontFamily: "monospace", color: "#a5f3fc" }}>server\setup-modal.ps1</span> (right-click → Run with PowerShell) to automatically create
                  your Modal account, deploy the model, and fill in the endpoint below.
                  Or paste your endpoint manually if you already have one.
                </div>
              </div>
            )}

            {cfg.analysis.enabled && (
              <div className="provider-fields">
                <Field
                  label="Inference Endpoint"
                  hint="Modal URL + API token separated by a space:  https://org--salenie-generate.modal.run  your-token"
                >
                  <input
                    value={cfg.analysis.inferenceEndpoint}
                    onChange={e => set("analysis.inferenceEndpoint", e.target.value)}
                    placeholder="https://yourorg--salenie-generate.modal.run  your-api-token"
                  />
                </Field>
                <div className="field-row">
                  <Field label="Whisper Model">
                    <select value={cfg.analysis.whisperModel} onChange={e => set("analysis.whisperModel", e.target.value)}>
                      <option value="tiny">Tiny (fastest)</option>
                      <option value="base">Base (recommended)</option>
                      <option value="small">Small (accurate)</option>
                      <option value="medium">Medium (best)</option>
                    </select>
                  </Field>
                  <Field label="STT Port" hint="Default: 8765">
                    <input
                      type="number"
                      value={cfg.analysis.sttPort}
                      onChange={e => set("analysis.sttPort", Number(e.target.value))}
                      placeholder="8765"
                    />
                  </Field>
                </div>
                <div style={{ fontSize: 10, color: "#475569", marginBottom: 12, lineHeight: 1.5 }}>
                  💡 The Whisper speech-to-text server is bundled inside the app — no Python required.
                  It starts automatically when AI Analysis is enabled.
                </div>
                <ServiceStatusRow config={cfg} />
              </div>
            )}
          </div>
        )}
      </div>

      {/* Footer */}
      <div className="ledger-footer">
        <button className="btn-cancel" onClick={handleClose}>Close</button>
        <button className="btn-save" onClick={handleSave} disabled={saving}>
          {saving ? "Saving…" : "Save Configuration"}
        </button>
      </div>
    </div>
  );
}

function ServiceStatusRow({ config }) {
  const [status, setStatus]     = useState(null);
  const [checking, setChecking] = useState(false);

  const check = async () => {
    setChecking(true);
    try {
      const result = await invoke("check_analysis_services", { config });
      setStatus(result);
    } catch {
      setStatus({ stt: false, ollama: false, model: false });
    } finally {
      setChecking(false);
    }
  };

  return (
    <div style={{ marginTop: 12 }}>
      <button className="btn-test" onClick={check} disabled={checking}>
        {checking ? "Checking…" : "Test Services"}
      </button>
      {status && (
        <div className="service-status" style={{ marginTop: 10 }}>
          {[
            { ok: status.stt,    label: "Whisper STT server", detail: `port ${config.analysis.sttPort}` },
            { ok: status.ollama, label: "Modal inference",     detail: "endpoint reachable" },
            { ok: status.model,  label: "Model ready",         detail: "container warm" },
          ].map(({ ok, label, detail }) => (
            <div className="service-status-row" key={label}>
              <div className={`status-dot ${ok ? "ok" : "err"}`} />
              <span style={{ fontWeight: 600 }}>{label}</span>
              <span style={{ color: "#475569", marginLeft: 4 }}>— {detail}</span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
