use candle_core::Tensor;

pub trait Tokenize {
    fn tokenize(self, data: String) -> Vec<Tensor>;
}