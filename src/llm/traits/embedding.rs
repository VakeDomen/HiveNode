use candle_core::Tensor;


pub trait Embed {
    fn embed(&self, data: Tensor) -> Tensor;
}