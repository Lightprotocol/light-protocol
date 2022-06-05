# Light-SDK

## Overview
![Light-SDK Overview](https://user-images.githubusercontent.com/32839416/167398127-106a46cd-8c6a-4c0f-88af-e4013b816d3c.png)


## Setup
1. Clone the repository

In the root folder of the repository
2. Run ```npm i``` 
3. Run ```npx tsc -w``` ( This will automatically transpile changed made in .ts files into .js )

## To develope and test the npm package locally
- Run ```npm link``` in the root folder of the repository
- Clone and setup the [Widget repository](https://github.com/Lightprotocol/widgets)
- In the root folder of the Widget repository run ```npm link PACKAGENAME``` ( the packagename is in the package.json of the SDK )

## To run the tests 
Run ```npm run test```
