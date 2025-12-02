PROJECT_ID=$(gcloud config get-value project)
REGION=europe-west1
REPO_NAME=light

docker buildx build --platform linux/amd64,linux/arm64 -f Dockerfile.light -t prover-light:latest .

docker tag prover-light:latest \
  $REGION-docker.pkg.dev/$PROJECT_ID/$REPO_NAME/prover-light:latest

docker push \
  $REGION-docker.pkg.dev/$PROJECT_ID/$REPO_NAME/prover-light:latest
