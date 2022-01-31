const path = require("path");
const CopyPlugin = require("copy-webpack-plugin");
const WasmPackPlugin = require("@wasm-tool/wasm-pack-plugin");

const dist = path.resolve(__dirname, "dist");

module.exports = {
  mode: "production",
  entry: {
    index: "./js/bootstrap.js"
  },
  output: {
    path: dist,
    filename: "[name].js"
  },
  devServer: {
    static: './dist',
  },
	experiments: { syncWebAssembly: true },
  plugins: [

    new WasmPackPlugin({
      crateDirectory: __dirname,
    }),
  ],
  performance: {
      // Disable warning for large wasm modules 
      hints: false,
      maxEntrypointSize: 512000,
      maxAssetSize: 512000
  }
};
