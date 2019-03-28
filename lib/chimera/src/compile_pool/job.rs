use super::pool::Compiler;
use crate::{
    alloc_pool::{AllocId, AllocPool},
    code::Code,
    llvm,
    module_ctx::{ModuleContext, SharedFunctionBody},
};
use futures::{
    channel::oneshot::{self, Receiver, Sender},
    future::{Future, FutureExt},
};
use std::sync::Arc;
use wasmer_runtime_core::{module::ModuleInfo, types::LocalFuncIndex};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Priority {
    /// Get to it at somepoint.
    Cold,
    /// Sooner rather than later.
    Warm,
    /// ASAP.
    Hot,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Mode {
    Baseline,
    Medium,
    High(llvm::Opt),
}

/// A `Job` represents the compilation of a function that will
/// complete sometime in the future.
pub struct Job {
    priority: Priority,
    mode: Mode,
    func_index: LocalFuncIndex,
    func_body: SharedFunctionBody,
    info: Arc<ModuleInfo>,
    alloc_pool: Arc<AllocPool>,
    sender: Sender<Result<AllocId<Code>, String>>,
}

impl Job {
    pub fn create(
        info: Arc<ModuleInfo>,
        alloc_pool: Arc<AllocPool>,
        func_body: SharedFunctionBody,
        func_index: LocalFuncIndex,
        priority: Priority,
        mode: Mode,
    ) -> impl Future<Output = Result<AllocId<Code>, String>> {
        // Create an async, oneshot channel that will receive
        // the compiled code at some point™️.
        let (sender, receiver) = oneshot::channel();

        Compiler.inject(Job {
            priority,
            mode,
            func_index,
            func_body,
            info,
            alloc_pool,
            sender,
        });

        receiver.map(|f| f.expect("the receiver has closed itself somehow, this shouldn't happen"))
    }

    pub(crate) fn do_compile(self) {
        use crate::code::Metadata;

        let code_id_res = match self.mode {
            Mode::Baseline => unimplemented!("compile with dynasm"),
            Mode::Medium => unimplemented!("compile with clif"),
            Mode::High(opt) => llvm::compile_function(
                &self.alloc_pool,
                &self.info,
                self.func_index,
                self.func_body.body(),
                opt,
            ),
        }
        .map_err(|e| format!("{:?}", e));

        // Ignore the result. In the future, we may want
        // to do something when we realize that the reciever
        // has closed itself.
        let _ = self.sender.send(code_id_res);
    }

    pub fn priority(&self) -> Priority {
        self.priority
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create() {
        use wasmer_runtime_core::structures::TypedIndex;
        let alloc_pool = Arc::new(AllocPool::new());
        let func_index = LocalFuncIndex::new(0);

        let future_code = Job::create(alloc_pool, func_index, Priority::Hot, Mode::Baseline);

        assert!(futures::executor::block_on(future_code).is_ok());
    }
}
