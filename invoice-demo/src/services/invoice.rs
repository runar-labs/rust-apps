use anyhow::Result;
use chrono::{DateTime, Utc};
use kagi_macros::{service, action};
use kagi_node::services::{ServiceRequest, ServiceResponse, RequestContext};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoiceItem {
    pub description: String,
    pub quantity: f64,
    pub unit_price: f64,
    pub amount: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Invoice {
    pub id: String,
    pub user_id: String,
    pub customer_name: String,
    pub customer_email: String,
    pub items: Vec<InvoiceItem>,
    pub subtotal: f64,
    pub tax_rate: f64,
    pub tax_amount: f64,
    pub total: f64,
    pub notes: Option<String>,
    pub due_date: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub status: InvoiceStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InvoiceStatus {
    Draft,
    Sent,
    Paid,
    Overdue,
    Cancelled,
}

#[service(name = "invoice", description = "Invoice management service")]
pub struct InvoiceService {
    invoices: Arc<RwLock<HashMap<String, Invoice>>>,
}

impl InvoiceService {
    pub fn new() -> Self {
        Self {
            invoices: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    #[action(operation = "create", description = "Create a new invoice")]
    async fn create_invoice(&self, _context: &RequestContext, request: ServiceRequest) -> Result<ServiceResponse> {
        let user_id = request.get_string("user_id")?;
        let customer_name = request.get_string("customer_name")?;
        let customer_email = request.get_string("customer_email")?;
        let items: Vec<InvoiceItem> = request.get_json("items")?;
        let tax_rate = request.get_f64("tax_rate")?;
        let notes = request.get_string_optional("notes")?;
        let due_date: DateTime<Utc> = request.get_datetime("due_date")?;

        let subtotal: f64 = items.iter().map(|item| item.amount).sum();
        let tax_amount = subtotal * tax_rate;
        let total = subtotal + tax_amount;

        let now = Utc::now();
        let invoice = Invoice {
            id: Uuid::new_v4().to_string(),
            user_id,
            customer_name,
            customer_email,
            items,
            subtotal,
            tax_rate,
            tax_amount,
            total,
            notes,
            due_date,
            created_at: now,
            updated_at: now,
            status: InvoiceStatus::Draft,
        };

        let mut invoices = self.invoices.write().await;
        invoices.insert(invoice.id.clone(), invoice.clone());

        Ok(ServiceResponse::json(serde_json::json!(invoice)))
    }

    #[action(operation = "get", description = "Get invoice by ID")]
    async fn get_invoice(&self, _context: &RequestContext, request: ServiceRequest) -> Result<ServiceResponse> {
        let invoice_id = request.get_string("invoice_id")?;

        let invoices = self.invoices.read().await;
        match invoices.get(&invoice_id) {
            Some(invoice) => Ok(ServiceResponse::json(serde_json::json!(invoice))),
            None => Ok(ServiceResponse::error("Invoice not found")),
        }
    }

    #[action(operation = "list", description = "List user's invoices")]
    async fn list_invoices(&self, _context: &RequestContext, request: ServiceRequest) -> Result<ServiceResponse> {
        let user_id = request.get_string("user_id")?;

        let invoices = self.invoices.read().await;
        let user_invoices: Vec<&Invoice> = invoices
            .values()
            .filter(|invoice| invoice.user_id == user_id)
            .collect();

        Ok(ServiceResponse::json(serde_json::json!(user_invoices)))
    }

    #[action(operation = "update", description = "Update invoice")]
    async fn update_invoice(&self, _context: &RequestContext, request: ServiceRequest) -> Result<ServiceResponse> {
        let invoice_id = request.get_string("invoice_id")?;
        let customer_name = request.get_string_optional("customer_name")?;
        let customer_email = request.get_string_optional("customer_email")?;
        let items: Option<Vec<InvoiceItem>> = request.get_json_optional("items")?;
        let tax_rate = request.get_f64_optional("tax_rate")?;
        let notes = request.get_string_optional("notes")?;
        let due_date: Option<DateTime<Utc>> = request.get_datetime_optional("due_date")?;
        let status: Option<InvoiceStatus> = request.get_json_optional("status")?;

        let mut invoices = self.invoices.write().await;
        let invoice = invoices.get_mut(&invoice_id).ok_or_else(|| anyhow::anyhow!("Invoice not found"))?;

        if let Some(name) = customer_name {
            invoice.customer_name = name;
        }
        if let Some(email) = customer_email {
            invoice.customer_email = email;
        }
        if let Some(new_items) = items {
            invoice.items = new_items;
            invoice.subtotal = invoice.items.iter().map(|item| item.amount).sum();
            invoice.tax_amount = invoice.subtotal * invoice.tax_rate;
            invoice.total = invoice.subtotal + invoice.tax_amount;
        }
        if let Some(rate) = tax_rate {
            invoice.tax_rate = rate;
            invoice.tax_amount = invoice.subtotal * rate;
            invoice.total = invoice.subtotal + invoice.tax_amount;
        }
        if let Some(new_notes) = notes {
            invoice.notes = Some(new_notes);
        }
        if let Some(new_due_date) = due_date {
            invoice.due_date = new_due_date;
        }
        if let Some(new_status) = status {
            invoice.status = new_status;
        }

        invoice.updated_at = Utc::now();

        Ok(ServiceResponse::json(serde_json::json!(invoice)))
    }

    #[action(operation = "delete", description = "Delete invoice")]
    async fn delete_invoice(&self, _context: &RequestContext, request: ServiceRequest) -> Result<ServiceResponse> {
        let invoice_id = request.get_string("invoice_id")?;

        let mut invoices = self.invoices.write().await;
        invoices.remove(&invoice_id);

        Ok(ServiceResponse::success("Invoice deleted successfully"))
    }
} 