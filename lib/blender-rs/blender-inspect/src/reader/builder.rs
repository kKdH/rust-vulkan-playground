use crate::parse::Identifier;
use crate::reader::Reader;

pub struct NoIdentifier;

pub struct ReaderBuilder<I> {
    identifier: I
}

impl ReaderBuilder<NoIdentifier> {

    pub fn new() -> Self {
        ReaderBuilder {
            identifier: NoIdentifier
        }
    }

    pub fn structures(&mut self, identifier: Identifier) -> ReaderBuilder<Identifier> {
        ReaderBuilder {
            identifier
        }
    }
}

impl ReaderBuilder<Identifier> {
    pub fn build(&self) -> Reader {
        Reader {}
    }
}

#[cfg(test)]
mod test {
    use crate::parse::Identifier;
    use crate::reader::builder::ReaderBuilder;

    #[test]
    fn test_build() {

        let reader = ReaderBuilder::new()
            .structures(Identifier::ME)
            .build();
    }
}
