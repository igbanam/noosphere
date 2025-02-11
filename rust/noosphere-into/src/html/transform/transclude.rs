use anyhow::Result;
use horrorshow::html;
use noosphere_fs::SphereFs;
use noosphere_storage::interface::Store;
use ucan::crypto::KeyMaterial;

use crate::transclude::{Transclude, Transcluder};

/// Transforms a transclude into HTML
pub struct TranscludeToHtmlTransformer<'a, S, K>
where
    S: Store,
    K: KeyMaterial + Clone + 'static,
{
    transcluder: Transcluder<'a, S, K>,
}

impl<'a, S, K> TranscludeToHtmlTransformer<'a, S, K>
where
    S: Store,
    K: KeyMaterial + Clone + 'static,
{
    pub fn new(fs: &'a SphereFs<S, K>) -> Self {
        TranscludeToHtmlTransformer {
            transcluder: Transcluder::new(fs),
        }
    }

    pub async fn transform(&'a self, slug: &str) -> Result<Option<String>> {
        let transclude = self.transcluder.transclude(slug).await?;

        Ok(transclude.map(|Transclude::Text(text_transclude)| {
            html! {
                li(class="transclude-item") {
                    a(class="transclude-format-text", href=&text_transclude.href) {
                        @ if let Some(title) = &text_transclude.title {
                            span(class="title") : title
                        }

                        @ if let Some(excerpt) = &text_transclude.excerpt {
                            span(class="excerpt") : excerpt
                        }

                        span(class="link-text") : &text_transclude.link_text
                    }
                }
            }
            .to_string()
        }))
    }
}
