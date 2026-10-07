// What Vite gives for a file imported with `?url&inline`: a data URL.
declare module "*.wasm?url&inline" {
  const dataUrl: string
  export default dataUrl
}
