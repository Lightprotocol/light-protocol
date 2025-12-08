PROJECT_ID=$(gcloud config get-value project)
REGION=europe-west1
REPO_NAME=light
TAG="${1:-latest}"  # Usage: ./publish_prover.sh [version]

docker buildx build --platform linux/amd64,linux/arm64 -f Dockerfile.light -t prover-light:$TAG --load .
docker tag prover-light:$TAG $REGION-docker.pkg.dev/$PROJECT_ID/$REPO_NAME/prover-light:$TAG
docker push $REGION-docker.pkg.dev/$PROJECT_ID/$REPO_NAME/prover-light:$TAG

# Deploy to GKE
CLUSTER_NAME="prover-gcloud-500"
CLUSTER_ZONE="us-central1-a"
IMAGE="$REGION-docker.pkg.dev/$PROJECT_ID/$REPO_NAME/prover-light:$TAG"
echo "Deploying $IMAGE to GKE cluster: $CLUSTER_NAME"
gcloud container clusters get-credentials $CLUSTER_NAME --zone=$CLUSTER_ZONE --project=$PROJECT_ID
kubectl set image deployment/prover-universal prover=$IMAGE -n prover
kubectl rollout status deployment/prover-universal -n prover
