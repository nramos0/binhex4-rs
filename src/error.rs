#[derive(Debug)]
#[non_exhaustive]
pub enum DecodeError {
    BadFormat,
    BadRunLengthEncoding,
    CRCVerificationError(CRCVerificationError),
}

#[derive(Debug)]
pub enum CRCVerificationError {
    Header,
    Data,
    Resource,
}

#[derive(Debug)]
#[non_exhaustive]
pub enum EncodeError {
    FileNameTooLong,
    DataTooLarge,
    ResourceTooLarge,
}
