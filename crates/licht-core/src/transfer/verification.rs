use std::time::{Duration, Instant};

use crate::{DownloadPhase, DownloadProgress, InstallationContext, Result};

pub(crate) struct VerificationProgress {
    context: InstallationContext,
    progress: DownloadProgress,
    last_publish: Instant,
}

impl VerificationProgress {
    pub(crate) async fn start(context: &InstallationContext, total: usize) -> Result<Self> {
        let total = u32::try_from(total).map_err(std::io::Error::other)?;
        context.stage(DownloadPhase::Verifying).await?;
        let progress = DownloadProgress {
            finished: 0,
            failed: 0,
            total,
            bytes_done: 0,
            bytes_total: 0,
        };
        context.send(progress).await?;
        Ok(Self {
            context: context.clone(),
            progress,
            last_publish: Instant::now(),
        })
    }

    pub(crate) async fn checked(&mut self) -> Result<()> {
        self.context.checkpoint().await?;
        self.progress.finished += 1;
        if self.progress.finished == self.progress.total
            || self.last_publish.elapsed() >= Duration::from_millis(100)
        {
            self.context.send(self.progress).await?;
            self.last_publish = Instant::now();
        }
        tokio::task::yield_now().await;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn verification_publishes_real_file_totals_and_final_completion() {
        let (sender, mut incoming) = tokio::sync::mpsc::channel(16);
        let context = InstallationContext::from(sender);
        let mut verification = VerificationProgress::start(&context, 3)
            .await
            .expect("start");
        for _ in 0..3 {
            verification.checked().await.expect("checked file");
        }
        let first = incoming.try_recv().expect("initial total");
        assert_eq!((first.finished, first.total, first.bytes_total), (0, 3, 0));
        let mut last = first;
        while let Ok(progress) = incoming.try_recv() {
            assert!(progress.finished >= last.finished && progress.finished <= progress.total);
            last = progress;
        }
        assert_eq!((last.finished, last.total, last.bytes_total), (3, 3, 0));
    }

    #[tokio::test]
    async fn verification_does_not_advance_after_cancellation() {
        let context = InstallationContext::default();
        let mut verification = VerificationProgress::start(&context, 3)
            .await
            .expect("start");
        context.control.cancel();
        assert!(matches!(
            verification.checked().await,
            Err(crate::CoreError::DownloadCancelled)
        ));
        assert_eq!(verification.progress.finished, 0);
    }
}
