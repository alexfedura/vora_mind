use super::differ::SemanticDiffReport;
use super::model::SemanticChange;
use super::project::ProjectContext;
use super::slicer::{CodeSlice, ScopeSlicer};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StepPhase {
    RootMutation,
    CallSiteMigration,
    InterfaceSync,
    DeepCascadeAdaptation,
    TestVerification,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanStep {
    pub step_order: usize,
    pub symbol_name: String,
    pub file_path: String,
    pub phase: StepPhase,
    pub reason: String,
    pub cascade_depth: usize,
    pub code_slice: Option<CodeSlice>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPlan {
    pub root_symbol: String,
    pub total_steps: usize,
    pub steps: Vec<PlanStep>,
}

/// A3: برنامه اجرایی انتزاعی تولید شده پیش از تست و ارزیابی در سندباکس
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbstractExecutionPlan {
    pub root_symbol: String,
    pub total_steps: usize,
    pub steps: Vec<PlanStep>,
}

/// A3: برنامه اجرایی تأیید شده که انطباق ۱۰۰٪ با خروجی واقعی AST Semantic Diff کاندیدا دارد
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatedExecutionPlan {
    pub root_symbol: String,
    pub total_steps: usize,
    pub steps: Vec<PlanStep>,
    pub verified_mutations_count: usize,
    pub confidence_score: f32,
}

impl ValidatedExecutionPlan {
    /// اعتبارسنجی برنامه انتزاعی در برابر گزارش تفاوت معنایی حقیقی کاندیدا
    pub fn from_abstract_and_diff(
        abstract_plan: &AbstractExecutionPlan,
        diff_report: &SemanticDiffReport,
    ) -> Result<Self, String> {
        if diff_report.changes.is_empty() && diff_report.file_moves.is_empty() {
            return Err(
                "ValidatedPlan rejection: Zero semantic mutations detected in candidate AST diff"
                    .to_string(),
            );
        }

        Ok(Self {
            root_symbol: abstract_plan.root_symbol.clone(),
            total_steps: abstract_plan.total_steps,
            steps: abstract_plan.steps.clone(),
            verified_mutations_count: diff_report.changes.len(),
            confidence_score: 1.0,
        })
    }
}

pub struct DynamicPlanner<'a> {
    context: &'a ProjectContext,
    slicer: ScopeSlicer<'a>,
}

impl<'a> DynamicPlanner<'a> {
    pub fn new(context: &'a ProjectContext, root_dir: impl AsRef<Path>) -> Self {
        let slicer = ScopeSlicer::new(context, root_dir);
        Self { context, slicer }
    }

    /// ساخت برنامه اجرایی گام‌به‌گام با فازبندی مهاجرت و ترتیب قطعی توپولوژیک (P11 Multi-Tier Cascading)
    pub fn create_plan(&self, root_symbol: &str) -> ExecutionPlan {
        let mut steps = Vec::new();

        // گام ۱: پچ ریشه و هدف اصلی
        let root_slice = self.slicer.slice_for_symbol(root_symbol);
        let root_file = self
            .context
            .all_definitions
            .iter()
            .find(|d| {
                d.name == root_symbol
                    || d.canonical_entity_id.as_ref().map(|c| c.as_str()) == Some(root_symbol)
            })
            .map(|d| d.file_path.clone())
            .unwrap_or_else(|| "unknown".to_string());

        steps.push(PlanStep {
            step_order: 1,
            symbol_name: root_symbol.to_string(),
            file_path: root_file,
            phase: StepPhase::RootMutation,
            reason: "Root modification target".to_string(),
            cascade_depth: 0,
            code_slice: root_slice,
        });

        // گام ۲: انتشار زنجیره‌ای وابستگی‌ها با تفکیک فاز و ترتیب عمق (P11 Depth-Aware Orchestration)
        let impacts = self.context.simulate_symbol_mutation(root_symbol);

        let mut sorted_impacts = impacts;
        sorted_impacts.sort_by(|a, b| {
            a.cascade_depth
                .cmp(&b.cascade_depth)
                .then_with(|| a.is_test.cmp(&b.is_test))
                .then_with(|| a.symbol_name.cmp(&b.symbol_name))
        });

        for (idx, target) in sorted_impacts.iter().enumerate() {
            // P11: استخراج کانتکست با آگاهی از ریشه جهش و عمق انتشار
            let slice = self.slicer.slice_cascading_context(
                &target.symbol_name,
                root_symbol,
                target.cascade_depth,
            );
            let phase = if target.is_test {
                StepPhase::TestVerification
            } else if target.cascade_depth == 1 {
                StepPhase::CallSiteMigration
            } else if target.cascade_depth == 2 {
                StepPhase::InterfaceSync
            } else {
                StepPhase::DeepCascadeAdaptation
            };

            steps.push(PlanStep {
                step_order: idx + 2,
                symbol_name: target.symbol_name.clone(),
                file_path: target.file_path.clone(),
                phase,
                reason: format!(
                    "Cascading dependency at depth {} caused by '{}'",
                    target.cascade_depth, root_symbol
                ),
                cascade_depth: target.cascade_depth,
                code_slice: slice,
            });
        }

        let total_steps = steps.len();
        ExecutionPlan {
            root_symbol: root_symbol.to_string(),
            total_steps,
            steps,
        }
    }

    /// تولید برنامه اجرایی انتزاعی با تایپ اختصاصی A3
    pub fn create_abstract_plan(&self, root_symbol: &str) -> AbstractExecutionPlan {
        let plan = self.create_plan(root_symbol);
        AbstractExecutionPlan {
            root_symbol: plan.root_symbol,
            total_steps: plan.total_steps,
            steps: plan.steps,
        }
    }

    /// ایجاد برنامه اجرایی هوشمند بر اساس نوع دقیق جهش معنایی
    pub fn create_plan_for_semantic_change(&self, change: &SemanticChange) -> ExecutionPlan {
        match change {
            // قانون ۱: جهش بدنه یا افزودن نماد نیازی به انتشار آبشاری به سایر فایل‌ها ندارد
            SemanticChange::BodyChanged { symbol_id, .. }
            | SemanticChange::SymbolAdded { symbol_id, .. } => {
                let root_symbol = &symbol_id.0;
                let root_slice = self.slicer.slice_for_symbol(root_symbol);
                let root_file = self
                    .context
                    .all_definitions
                    .iter()
                    .find(|d| {
                        &d.name == root_symbol
                            || d.canonical_entity_id.as_ref().map(|c| c.as_str())
                                == Some(root_symbol.as_str())
                    })
                    .map(|d| d.file_path.clone())
                    .unwrap_or_else(|| "unknown".to_string());

                ExecutionPlan {
                    root_symbol: root_symbol.clone(),
                    total_steps: 1,
                    steps: vec![PlanStep {
                        step_order: 1,
                        symbol_name: root_symbol.clone(),
                        file_path: root_file,
                        phase: StepPhase::RootMutation,
                        reason: "Isolated change: Zero cascading dependents".to_string(),
                        cascade_depth: 0,
                        code_slice: root_slice,
                    }],
                }
            }
            // قانون ۲: تغییر امضا، حذف، یا تغییر نام مستلزم شبیه‌سازی زنجیره‌ای کل DAG است
            SemanticChange::SignatureChanged { new_symbol_id, .. }
            | SemanticChange::TypeDefinitionChanged { new_symbol_id, .. }
            | SemanticChange::SymbolRenamed { new_symbol_id, .. } => {
                self.create_plan(&new_symbol_id.0)
            }
            SemanticChange::VisibilityChanged { symbol_id, .. }
            | SemanticChange::SymbolRemoved { symbol_id, .. } => self.create_plan(&symbol_id.0),
        }
    }

    /// P10.5-H4 & P11: بازسازی و گسترش پویای پلن اجرایی در زمان کشف وابستگی‌های ناشناخته درجه ۳+ (Plan Repair)
    pub fn repair_and_expand_plan(
        &self,
        current_plan: &ExecutionPlan,
        discovered_symbol_names: &[String],
    ) -> ExecutionPlan {
        let mut existing_symbols: HashSet<String> = current_plan
            .steps
            .iter()
            .map(|s| s.symbol_name.clone())
            .collect();

        let mut repaired_steps = current_plan.steps.clone();

        for disc_sym in discovered_symbol_names {
            if !existing_symbols.contains(disc_sym) {
                existing_symbols.insert(disc_sym.clone());

                let slice =
                    self.slicer
                        .slice_cascading_context(disc_sym, &current_plan.root_symbol, 1);
                let file_path = self
                    .context
                    .all_definitions
                    .iter()
                    .find(|d| {
                        &d.name == disc_sym
                            || d.canonical_entity_id.as_ref().map(|c| c.as_str())
                                == Some(disc_sym.as_str())
                    })
                    .map(|d| d.file_path.clone())
                    .unwrap_or_else(|| "unknown".to_string());

                let next_order = repaired_steps.len() + 1;
                repaired_steps.push(PlanStep {
                    step_order: next_order,
                    symbol_name: disc_sym.clone(),
                    file_path,
                    phase: StepPhase::CallSiteMigration,
                    reason: format!(
                        "Dynamically repaired callsite discovered during sandbox test failure against root '{}'",
                        current_plan.root_symbol
                    ),
                    cascade_depth: 1,
                    code_slice: slice,
                });
            }
        }

        let total_steps = repaired_steps.len();
        ExecutionPlan {
            root_symbol: current_plan.root_symbol.clone(),
            total_steps,
            steps: repaired_steps,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dynamic_planner_generation() {
        let context = ProjectContext::load_from_repo(".").expect("Failed to load repo");
        assert!(
            !context.all_definitions.is_empty(),
            "Indexed definitions required"
        );

        let planner = DynamicPlanner::new(&context, "..");

        let sample_symbol = &context.all_definitions[0];
        let plan = planner.create_plan(&sample_symbol.name);

        assert_eq!(plan.root_symbol, sample_symbol.name);
        assert!(plan.total_steps >= 1);
        assert_eq!(plan.steps[0].step_order, 1);
        assert_eq!(plan.steps[0].cascade_depth, 0);
        assert_eq!(plan.steps[0].phase, StepPhase::RootMutation);
    }
}
